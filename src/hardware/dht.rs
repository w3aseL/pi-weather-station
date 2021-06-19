// Credit where credit's due for sensor:
// https://github.com/jackmead515/rust_dht11/blob/master/src/dht11.rs

use rppal::gpio::{ IoPin, Mode, PullUpDown };
use std::thread::{ sleep, spawn };
use std::time::{ Duration, SystemTime };
use crossbeam_channel::{ Sender };
use crate::hardware::events::{ Event, EventType, Payload };
use chrono::{ DateTime };
use chrono::offset::{ Utc };
use crate::data::process::{ DataPoint };

#[derive(Debug, Clone, Copy)]
pub struct DHTData {
    temperature: f32,
    humidity: f32,
    last_updated: Option<SystemTime>
}

impl DHTData {
    pub fn new(temp: f32, humidity: f32, last_updated: Option<SystemTime>) -> Self {
        Self {
            temperature: temp,
            humidity,
            last_updated
        }
    }

    pub fn is_valid(&self) -> bool {
        self.last_updated.is_some()
    }

    pub fn get_temp_celsius(&self) -> f32 {
        self.temperature
    }

    pub fn get_temp_farenheit(&self) -> f32 {
        self.get_temp_celsius() * 9.0 / 5.0 + 32.0
    }

    pub fn get_humidity(&self) -> f32 {
        self.humidity
    }

    pub fn get_last_updated(&self) -> Option<SystemTime> {
        self.last_updated
    }
}

pub struct DHTPayload {
    data: DHTData
}

impl DHTPayload {
    pub fn new(temp: f32, humidity: f32, last_updated: Option<SystemTime>) -> Self {
        DHTPayload {
            data: DHTData::new(temp, humidity, last_updated)
        }
    }
}

impl Payload for DHTPayload {
    fn send_message(&self) {
        // ...
    }

    fn update_data_fields(&self, data: &mut DataPoint) {
        data.update_dht(self.data.clone())
    }
}

const PI_CLOCK: u64 = 1_500_000_000;

pub enum DHTState {
    Ok = 0,
    ErrorChecksum = -1,
    ErrorTimeout = -2,
    InvalidValue = -999
}

impl DHTState {
    pub fn get_state_str(state: Self) -> String {
        match state {
            DHTState::Ok => "Ok".to_string(),
            DHTState::ErrorChecksum => "Checksum".to_string(),
            DHTState::ErrorTimeout => "Timeout".to_string(),
            DHTState::InvalidValue => "Invalid Value".to_string()
        }
    }

    pub fn get_state_from_code(code: i32) -> Self {
        match code {
            0 => DHTState::Ok,
            -1 => DHTState::ErrorChecksum,
            -2=> DHTState::ErrorTimeout,
            -999 => DHTState::InvalidValue,
            _ => DHTState::InvalidValue
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[warn(dead_code)]
pub enum DHTSensor {
    DHT11 = 18,
    DHT22 = 1
}

pub struct DHT {
    pin: IoPin,
    sensor_type: DHTSensor,
    humidity: f32,
    temp: f32,
    event_sender: Sender<Event>,
    payload_sender: Sender<Box<dyn Payload>>,
    last_update: Option<SystemTime>
}

impl DHT {
    pub fn new(pin: IoPin, sensor_type: DHTSensor, event_sender: Sender<Event>, payload_sender: Sender<Box<dyn Payload>>) -> Self {
        Self {
            pin,
            sensor_type,
            humidity: 0.0,
            temp: 0.0,
            event_sender,
            payload_sender,
            last_update: None
        }
    }

    pub fn start_reading(mut self) {
        self.pin.set_pullupdown(PullUpDown::Off);

        spawn(move || {
            let mut success_counter = 0; let mut error_counter = 0;

            loop {
                match self.update() {
                    Ok(_) => {
                        //println!("Temperature: {}°F ({}°C)", self.get_temp_farenheit(), self.get_temp_celsius());
                        //println!("Humidity: {}%", self.get_humidity());
                        success_counter += 1;

                        self.last_update = Some(SystemTime::now());

                        self.event_sender.send(Event::new(EventType::SensorRead)).unwrap();
                        self.payload_sender.send(Box::new(DHTPayload::new(self.temp, self.humidity, self.last_update)));
                    }
                    Err(code) => {
                        println!("Failed to read from sensor! Code: {}", DHTState::get_state_str(DHTState::get_state_from_code(code)));
                        error_counter += 1;
                    }
                }

                if success_counter + error_counter == 20 {
                    let mut last_update_str = "N/A".to_string();

                    if let Some(time) = self.last_update {
                        let date_time: DateTime<Utc> = time.into();

                        last_update_str = date_time.format("%d/%m/%Y %T").to_string();
                    }

                    println!("DHT11 Test:\nSuccess: {}\nError: {}\nLast Update: {}", success_counter, error_counter, last_update_str);
                    break;
                }

                sleep(Duration::from_secs(2));
            }

            sleep(Duration::from_secs(10));

            self.event_sender.send(Event::new(EventType::Exit)).unwrap();
        });
    }

    fn update(&mut self) -> Result<(), i32> {
        let read_ret = self.read_sensor(self.sensor_type as u8);

        if read_ret != DHTState::Ok as i32 {
            self.humidity = (DHTState::InvalidValue as i32) as f32;
            self.temp = (DHTState::InvalidValue as i32) as f32;
            return Err(read_ret)
        }

        Ok(())
    }

    fn read_sensor(&mut self, wake_up_delay: u8) -> i32 {
        // Startup
        self.pin.set_mode(Mode::Output);
        self.pin.set_high();
        sleep(Duration::from_millis(100));
        self.pin.set_low();
        sleep(Duration::from_millis(wake_up_delay as u64 + 2));
        self.pin.set_high();
        sleep(Duration::from_micros(30));
        self.pin.set_mode(Mode::Input);

        // Max clock speed
        let max_count = (PI_CLOCK / 40000) as u32;

        const PULSES: usize = 41;

        let mut pulse_cnts = [0u32; PULSES*2];
        
        for i in (0..PULSES*2).step_by(2) {
            while self.pin.is_low() {
                pulse_cnts[i] += 1;
    
                if pulse_cnts[i] >= max_count { return DHTState::ErrorTimeout as i32; }
            }

            while self.pin.is_high() {
                pulse_cnts[i+1] += 1;
    
                if pulse_cnts[i+1] >= max_count { return DHTState::ErrorTimeout as i32; }
            }
        }

        let mut threshold = 0;
        for i in (2..PULSES).step_by(2) {
            threshold += pulse_cnts[i];
        }
        threshold /= PULSES as u32 - 1;

        let mut data = [0u8; 5];

        for i in (3..PULSES * 2).step_by(2) {
            let idx = (i - 3) / 16;
            data[idx] <<= 1;

            if pulse_cnts[i] >= threshold {
                data[idx] |= 1;
            }
        }

        self.humidity = (data[0] as f32) + ((data[1] as f32) * 0.1);
        self.temp = (data[2] as f32) + ((data[3] as f32) * 0.1);

        let mut checksum = 0u8;

        for i in 0..4 { checksum += data[i] }

        if checksum != data[4] {
            return DHTState::ErrorChecksum as i32;
        }

        return DHTState::Ok as i32;
    }

    pub fn get_temp_celsius(&self) -> f32 {
        self.temp
    }

    pub fn get_temp_farenheit(&self) -> f32 {
        self.get_temp_celsius() * 9.0 / 5.0 + 32.0
    }

    pub fn get_humidity(&self) -> f32 {
        self.humidity
    }
}