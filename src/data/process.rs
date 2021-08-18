use crossbeam_channel::{ Sender, Receiver };
use std::error::{ Error };
use std::thread::{ sleep, spawn };
use std::time::{ Duration, SystemTime };
use chrono::{ DateTime, Date, Local };
use chrono::offset::{ Utc };
use sysinfo::{ ProcessorExt, System, SystemExt };
use postgres::{ Client };

use crate::config::{ Config };
use crate::db::{ get_client };
use crate::hardware::events::{ Event, EventType, Payload };
use crate::hardware::dht::{ DHTData };
use crate::hardware::anemometer::{ AnemometerData };
use crate::hardware::vane::{ WindVaneData };
use crate::hardware::rain::{ RainData };
use crate::hardware::display::{ LCDDisplay };

use super::DatabaseType;
use super::types::{ Rain };

const DISPLAY_RS_PIN: u8 = 13;
const DISPLAY_EN_PIN: u8 = 19;
const DISPLAY_D_PINS: [u8; 4] = [ 12, 16, 20, 21 ];
const DISPLAY_ROWS: usize = 2;
const DISPLAY_COLS: usize = 16;

#[derive(Clone)]
pub struct DataPoint {
    dht_data: DHTData,
    anemometer_data: AnemometerData,
    directional_data: WindVaneData,
    rain_data: RainData
}

impl DataPoint { 
    pub fn new() -> Self {
        Self {
            dht_data: DHTData::new(-999.0, -999.0, None),
            anemometer_data: AnemometerData::new(0.0, None),
            directional_data: WindVaneData::new(0.0, None),
            rain_data: RainData::new(0, 0.0, None)
        }
    }

    pub fn update_dht(&mut self, data: DHTData) {
        self.dht_data = data;
    }

    pub fn update_anemometer(&mut self, data: AnemometerData) {
        self.anemometer_data = data;
    }

    pub fn update_direction(&mut self, data: WindVaneData) {
        self.directional_data = data;
    }

    pub fn update_rain(&mut self, data: RainData) {
        self.rain_data = data;
    }

    pub fn print_data(&self) {
        let mut data_str = "".to_string();

        if self.dht_data.is_valid() {
            let time: DateTime<Utc> = self.dht_data.get_last_updated().unwrap().into();

            data_str.push_str(format!("Temperature: {}°F ({}°C) -- Humidity: {}% (Last Updated: {})\n", self.dht_data.get_temp_farenheit(), self.dht_data.get_temp_celsius(), self.dht_data.get_humidity(), time.format("%d/%m/%Y %T").to_string()).as_str());
        }

        if self.anemometer_data.is_valid() {
            let time: DateTime<Utc> = self.anemometer_data.get_last_updated().unwrap().into();

            data_str.push_str(format!("Wind Speed: {} MPH ({} km/h) -- (Last Updated: {})\n", self.anemometer_data.get_mph(), self.anemometer_data.get_kph(), time.format("%d/%m/%Y %T").to_string()).as_str());
        }

        if self.directional_data.is_valid() {
            let time: DateTime<Utc> = self.directional_data.get_last_updated().unwrap().into();

            data_str.push_str(format!("Wind Direction: {}° (Last Updated: {})\n", self.directional_data.get_direction(), time.format("%d/%m/%Y %T").to_string()).as_str())
        }

        if self.rain_data.is_valid() {
            let time: DateTime<Utc> = self.rain_data.get_last_updated().unwrap().into();

            data_str.push_str(format!("Rain Collected: {}in ({}cm) -- (Last Updated: {})\n", self.rain_data.get_amount_in(), self.rain_data.get_amount_in(), time.format("%d/%m/%Y %T").to_string()).as_str());
        }

        if data_str.len() > 0 { print!("{}", data_str); }
    }
}

fn ping() -> bool {
    let res = reqwest::blocking::get("http://detectportal.firefox.com/success.txt");

    if let Err(e) = res {
        return false;
    } else if let Ok(r) = res {
        if r.text().unwrap().eq("success".into()) {
            return true;
        }
    }

    return false;
}

#[derive(Debug, Clone, Copy)]
pub struct DaytimeData {
    date: Date<Local>,
    prev_date: Option<Date<Local>>,
    pub rain_total: u32,
    pub wind_max: f32,
    pub wind_min: f32,
    pub temp_hi: f32,
    pub temp_lo: f32,
    pub temp_avg: f32,
    pub temp_total: f32,
    pub temp_col_count: u32
}

impl DaytimeData {
    pub fn new(prev_date: Option<Date<Local>>) -> Self {
        Self {
            date: Local::today(),
            prev_date,
            rain_total: 0,
            wind_max: 0.0,
            wind_min: -1.0,
            temp_hi: 0.0,
            temp_lo: -1.0,
            temp_avg: 0.0,
            temp_total: 0.0,
            temp_col_count: 0
        }
    }

    pub fn get_rain_total_cm(&self) -> f32 {
        RainData::convert_to_cm(self.rain_total)
    }

    pub fn get_rain_total_in(&self) -> f32 {
        RainData::convert_to_in(self.rain_total)
    }

    pub fn get_wind_min_mph(&self) -> f32 {
        AnemometerData::convert_to_mph(self.wind_min)
    }

    pub fn get_wind_max_mph(&self) -> f32 {
        AnemometerData::convert_to_mph(self.wind_max)
    }

    pub fn get_current_date(&self) -> Date<Local> {
        self.date
    }

    pub fn save_to_file(&self) {
        // TODO
    }
}

pub struct DataManager {
    config: Config,
    sender: Sender<Event>,
    receiver: Receiver<Box<dyn Payload>>,
    update_rcv: Receiver<Event>,
    data: DataPoint,
    lcd_display: LCDDisplay,
    system_info: System,
    db_client: Client,
    current_data: DaytimeData,
    has_internet_connection: bool
}

impl DataManager {
    pub fn new(sender: Sender<Event>, receiver: Receiver<Box<dyn Payload>>, update_rcv: Receiver<Event>, config: Config) -> Result<Self, Box<dyn Error>> {
        let mut client: Client;

        if config.is_prod_env() {
            client = get_client(config.prod.addr.clone(), config.prod.username.clone(), config.prod.password.clone(), config.prod.dbname.clone());
        } else {
            client = get_client(config.dev.addr.clone(), config.dev.username.clone(), config.dev.password.clone(), config.dev.dbname.clone());
        }

        Rain::create_table(&mut client);

        Ok(Self {
            config: config,
            sender,
            receiver,
            update_rcv,
            data: DataPoint::new(),
            lcd_display: LCDDisplay::new(DISPLAY_RS_PIN, DISPLAY_EN_PIN, DISPLAY_D_PINS, DISPLAY_COLS, DISPLAY_ROWS)?,
            system_info: System::new_all(),
            db_client: client,
            current_data: DaytimeData::new(None),
            has_internet_connection: ping()
        })
    }

    pub fn start(mut self) {
        spawn(move || {
            let mut lcd_loop = 0;
            let mut update_lcd = 5;         // Update on startup (updates LCD every 5 seconds to new state)

            let mut ping_loop = 0;

            loop {
                // self.sender.send(Event::new(EventType::UpdateData)).unwrap();
                let mut has_updated = false;

                sleep(Duration::from_millis(5));

                while !self.receiver.is_empty() {
                    let payload = self.receiver.recv().unwrap();

                    payload.update_data_fields(&mut self.data, &mut self.current_data);

                    has_updated = true;
                }

                sleep(Duration::from_millis(5));

                while !self.update_rcv.is_empty() {
                    let event = self.update_rcv.recv().unwrap();

                    match event.get_event_type() {
                        EventType::MidnightRefresh => {
                            self.current_data.save_to_file();

                            let prev_data = self.current_data;

                            self.current_data = DaytimeData::new(Some(prev_data.get_current_date()));
                        },
                        _ => {}
                    }
                }

                if has_updated { self.data.print_data(); }

                let mut elapsed = Duration::from_secs(0);

                if ping_loop < 30 {
                    ping_loop += 1;
                } else {
                    let time = SystemTime::now();

                    self.has_internet_connection = ping();
    
                    let elapsed_add = time.elapsed().unwrap();

                    if elapsed_add.as_millis() <= 900 {
                        elapsed += elapsed_add;
                    } else {
                        let secs = elapsed_add.as_secs();
    
                        update_lcd += secs;
                        ping_loop += secs;
                    }
                }

                if update_lcd < 5 {
                    update_lcd += 1;
                } else {
                    update_lcd = 0;

                    let time = SystemTime::now();

                    self.print_data_lcd(lcd_loop);
    
                    lcd_loop = if lcd_loop == 5 { 0 } else { lcd_loop + 1 };
    
                    elapsed += time.elapsed().unwrap();
                } 

                sleep(Duration::from_millis(990) - elapsed);
            }
        });
    }

    pub fn print_data_lcd(&mut self, show_id: i32) {
        self.lcd_display.clear();
        self.lcd_display.cursor_home();

        match show_id {
            0 => {
                let time: DateTime<Local> = Local::now();

                self.lcd_display.write_message(format!("   Pi Weather   \n {}", time.format("%m/%d/%y %H:%M").to_string()));
            },
            1 => {
                if self.data.dht_data.is_valid() {
                    self.lcd_display.write_message(format!("{:.1}°F ({:.1}°C)\n{:.1}% Humidity", self.data.dht_data.get_temp_farenheit(), self.data.dht_data.get_temp_celsius(), self.data.dht_data.get_humidity()));
                } else {
                    self. lcd_display.write_message(format!("Temp/Humidity\nunavailable!"));
                }
            },
            2 => {
                if self.data.anemometer_data.is_valid() && self.data.directional_data.is_valid() {
                    self.lcd_display.write_message(format!("{}° {}\n{:.1}mph {:.1}k/hr", self.data.directional_data.get_direction(), self.data.directional_data.get_dir_as_string(), self.data.anemometer_data.get_mph(), self.data.anemometer_data.get_kph()));
                } else {
                    self.lcd_display.write_message(format!("Wind data\nunavailable!"));
                }
            },
            3 => {
                if self.data.anemometer_data.is_valid() {
                    self.lcd_display.write_message(format!("Min: {:.1}mph\nMax: {:.1}mph", self.current_data.get_wind_min_mph(), self.current_data.get_wind_max_mph()));
                } else {
                    self.lcd_display.write_message(format!("Wind data\nunavailable!"));
                }
            },
            4 => {
                if self.data.rain_data.is_valid() {
                    self.lcd_display.write_message(format!("{:.2} in\n{:.2} cm", self.current_data.get_rain_total_in(), self.current_data.get_rain_total_cm()));
                } else {
                    self.lcd_display.write_message(format!("Rain data\nunavailable!"));
                }
            },
            5 => {
                self.system_info.refresh_system();

                self.lcd_display.write_message(format!("CPU: {:.1}%\nMem: {:.2}MB", self.system_info.get_global_processor_info().get_cpu_usage(), (self.system_info.get_used_memory() as f32) / 1000.0));
            },
            6 => {

            },
            7 => {

            },
            _ => {  }
        }
    }
}