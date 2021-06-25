use crossbeam_channel::{ Sender, Receiver };
use std::error::{ Error };
use std::thread::{ sleep, spawn };
use std::time::{ Duration, SystemTime };
use chrono::{ DateTime, Local };
use chrono::offset::{ Utc };
use sysinfo::{ ProcessorExt, System, SystemExt };

use crate::hardware::events::{ Event, EventType, Payload };
use crate::hardware::dht::{ DHTData };
use crate::hardware::anemometer::{ AnemometerData };
use crate::hardware::vane::{ WindVaneData };
use crate::hardware::rain::{ RainData };
use crate::hardware::display::{ LCDDisplay };

const DISPLAY_RS_PIN: u8 = 13;
const DISPLAY_EN_PIN: u8 = 19;
const DISPLAY_D_PINS: [u8; 4] = [ 12, 16, 20, 21 ];
const DISPLAY_ROWS: usize = 2;
const DISPLAY_COLS: usize = 16;

pub struct DataPoint {
    message: Option<String>,
    last_updated: Option<SystemTime>,
    dht_data: DHTData,
    anemometer_data: AnemometerData,
    directional_data: WindVaneData,
    rain_data: RainData
}

impl DataPoint { 
    pub fn new() -> Self {
        Self {
            message: None,
            last_updated: None,
            dht_data: DHTData::new(-999.0, -999.0, None),
            anemometer_data: AnemometerData::new(0.0, None),
            directional_data: WindVaneData::new(0.0, None),
            rain_data: RainData::new(0.0, None)
        }
    }

    pub fn update_message(&mut self, message: String) {
        self.message = Some(message);
        self.last_updated = Some(SystemTime::now());
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

        if self.message.is_some() && self.last_updated.is_some() {
            let time: DateTime<Utc> = self.last_updated.unwrap().into();

            data_str.push_str(format!("Data Report:\nMessage: \"{}\" (Last Updated: {})\n", self.message.clone().unwrap(), time.format("%d/%m/%Y %T").to_string()).as_str());
        }

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

    pub fn print_data_lcd(&self, show_id: i32, lcd_display: &mut LCDDisplay, system_info: &mut System) {
        lcd_display.clear();
        lcd_display.cursor_home();

        match show_id {
            0 => {
                let time: DateTime<Local> = Local::now();

                lcd_display.write_message(format!("   Pi Weather   \n {}", time.format("%m/%d/%y %H:%M").to_string()));
            },
            1 => {
                if self.dht_data.is_valid() {
                    lcd_display.write_message(format!("{:.1}°F ({:.1}°C)\n{:.1}% Humidity", self.dht_data.get_temp_farenheit(), self.dht_data.get_temp_celsius(), self.dht_data.get_humidity()));
                } else {
                    lcd_display.write_message(format!("Temp/Humidity\nunavailable!"));
                }
            },
            2 => {
                if self.anemometer_data.is_valid() && self.directional_data.is_valid() {
                    lcd_display.write_message(format!("{}° {}\n{:.1}mph {:.1}k/hr", self.directional_data.get_direction(), self.directional_data.get_dir_as_string(), self.anemometer_data.get_mph(), self.anemometer_data.get_kph()));
                } else {
                    lcd_display.write_message(format!("Wind data\nunavailable!"));
                }
            },
            3 => {
                if self.rain_data.is_valid() {
                    lcd_display.write_message(format!("{:.2} in/sec\n{:.2} cm/sec", self.rain_data.get_amount_in(), self.rain_data.get_amount_cm()));
                } else {
                    lcd_display.write_message(format!("Rain data\nunavailable!"));
                }
            },
            4 => {
                system_info.refresh_system();

                lcd_display.write_message(format!("CPU: {:.1}%\nMem: {:.2}MB", system_info.get_global_processor_info().get_cpu_usage(), (system_info.get_used_memory() as f32) / 1000.0));
            },
            _ => {  }
        }
    }
}

pub struct DataManager {
    sender: Sender<Event>,
    receiver: Receiver<Box<dyn Payload>>,
    data: DataPoint,
    lcd_display: LCDDisplay,
    system_info: System
}

impl DataManager {
    pub fn new(sender: Sender<Event>, receiver: Receiver<Box<dyn Payload>>) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            sender,
            receiver,
            data: DataPoint::new(),
            lcd_display: LCDDisplay::new(DISPLAY_RS_PIN, DISPLAY_EN_PIN, DISPLAY_D_PINS, DISPLAY_COLS, DISPLAY_ROWS)?,
            system_info: System::new_all()
        })
    }

    pub fn start(mut self) {
        spawn(move || {
            let mut lcd_loop = 0;
            let mut update_lcd = 5;         // Update on startup (updates LCD every 5 seconds to new state)

            loop {
                self.sender.send(Event::new(EventType::UpdateData)).unwrap();

                sleep(Duration::from_millis(5));

                while !self.receiver.is_empty() {
                    let payload = self.receiver.recv().unwrap();

                    payload.update_data_fields(&mut self.data);
                }

                sleep(Duration::from_millis(5));

                self.data.print_data();

                if update_lcd < 5 {
                    update_lcd += 1;

                    sleep(Duration::from_millis(990));
                } else {
                    update_lcd = 0;

                    let time = SystemTime::now();

                    self.data.print_data_lcd(lcd_loop, &mut self.lcd_display, &mut self.system_info);
    
                    lcd_loop = if lcd_loop == 4 { 0 } else { lcd_loop + 1 };
    
                    let elapsed = time.elapsed().unwrap();
    
                    sleep(Duration::from_millis(990) - elapsed);
                } 
            }
        });
    }
}