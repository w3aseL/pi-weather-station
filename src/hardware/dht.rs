// Credit where credit's due for sensor:
// https://github.com/jackmead515/rust_dht11/blob/master/src/dht11.rs
// https://github.com/RobTillaart/DHTstable/blob/master/DHTStable.cpp

use rppal::gpio::{ IoPin, Mode, PullUpDown };
use std::thread::{ sleep, spawn };
use std::time::{ Duration, SystemTime };
use crossbeam_channel::{ Sender };
use chrono::{ DateTime };
use chrono::offset::{ Utc };

use crate::hardware::events::{ Event, EventType, Payload };
use crate::data::process::{ DataPoint };

const MAX_CLOCKS: u32 = 32_000;

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

pub struct DHT {
    pin: IoPin,
    humidity: f32,
    temp: f32,
    event_sender: Sender<Event>,
    payload_sender: Sender<Box<dyn Payload>>,
    last_update: Option<SystemTime>
}

impl DHT {
    pub fn new(pin: IoPin, event_sender: Sender<Event>, payload_sender: Sender<Box<dyn Payload>>) -> Self {
        Self {
            pin,
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

                        self.payload_sender.send(Box::new(DHTPayload::new(self.temp, self.humidity, self.last_update))).unwrap();
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
        let read_ret = self.read_sensor();

        if read_ret != DHTState::Ok as i32 {
            self.humidity = (DHTState::InvalidValue as i32) as f32;
            self.temp = (DHTState::InvalidValue as i32) as f32;
            return Err(read_ret)
        }

        Ok(())
    }

    fn read_sensor(&mut self) -> i32 {
        let mut mask = 128u8;
        let mut idx = 0;

        // Startup
        self.pin.set_mode(Mode::Output);
        self.pin.set_high();
        sleep(Duration::from_millis(100));
        self.pin.set_low();
        sleep(Duration::from_micros(1100));
        self.pin.set_mode(Mode::Input);
        self.pin.set_pullupdown(PullUpDown::PullUp);
        sleep(Duration::from_micros(30));

        const PULSES: usize = 41;
        let mut data = [0u8; 5];

        /*
        let mut pulse_cnts = [0u32; PULSES*2];
        
        for i in (0..PULSES*2).step_by(2) {
            while self.pin.is_low() {
                pulse_cnts[i] += 1;
                if pulse_cnts[i] >= MAX_CLOCKS {
                    return DHTState::ErrorTimeout as i32; // Exceeded timeout, fail.
                }
            }

            while self.pin.is_high() {
                pulse_cnts[i+1] += 1;
                if pulse_cnts[i+1] >= MAX_CLOCKS {
                    return DHTState::ErrorTimeout as i32; // Exceeded timeout, fail.
                }
            }
        }

        let mut threshold = 0;
        for i in (2..PULSES).step_by(2) {
            threshold += pulse_cnts[i];
        }
        threshold /= PULSES as u32 - 1;

        for i in (3..PULSES * 2).step_by(2) {
            let idx = (i - 3) / 16;
            data[idx] <<= 1;

            if pulse_cnts[i] >= threshold {
                data[idx] |= 1;
            }
        }
        */

        let mut count = MAX_CLOCKS;
        while self.pin.is_low() {
            count -= 1;
            if count == 0 { return DHTState::ErrorTimeout as i32; }
        }

        count = MAX_CLOCKS;
        while self.pin.is_high() {
            count -= 1;
            if count == 0 { return DHTState::ErrorTimeout as i32; }
        }

        for _ in 0..PULSES-1 {
            count = MAX_CLOCKS;
                while self.pin.is_low() {
                count -= 1;
                if count == 0 { return DHTState::ErrorTimeout as i32; }
            }

            let time = SystemTime::now();

            count = MAX_CLOCKS;
            while self.pin.is_high() {
                count -= 1;
                if count == 0 { return DHTState::ErrorTimeout as i32; }
            }

            if time.elapsed().unwrap().as_micros() > 40 {
                data[idx] |= mask;
            }

            mask >>= 1;
            if mask == 0 {
                mask = 128;
                idx += 1;
            }
        }

        self.humidity = ((data[0] as i16) << 8 | (data[1] as i16)) as f32 * 0.1;
        self.temp = ((data[2] as i16) * 256 + (data[3] as i16)) as f32 * 0.1;

        let mut checksum = 0u8;

        // println!("Data bits: {:?}, humidity: {:.2}, temp: {:.2}", data, self.humidity, self.temp);

        for i in 0..4 {
            match checksum.checked_add(data[i]) {
                Some(val) => checksum = val,
                None => return DHTState::ErrorChecksum as i32
            }
        }

        if checksum != data[4] {
            return DHTState::ErrorChecksum as i32;
        }

        return DHTState::Ok as i32;
    }
}

#[cfg(test)]
mod test {
    use std::error::Error;
    use std::thread::sleep;
    use std::time::Duration;
    use rppal::gpio::{ Gpio };
    use crate::{ DHT, DHT_PIN, Mode };
    use crate::hardware::dht::{ DHTState };
    use crossbeam_channel as channel;

    #[test]
    fn test_temp() -> Result<(), Box<dyn Error>> {
        let (tx, _) = channel::unbounded();
        let (payload_tx, _) = channel::unbounded();

        let mut dht_sensor = DHT::new(Gpio::new()?.get(DHT_PIN)?.into_io(Mode::Input), tx.clone(), payload_tx.clone());

        let mut success = 0;

        for _ in 0..5 {
            match dht_sensor.update() {
                Ok(_) => {
                    success += 1;
                },
                Err(code) => {
                    println!("Failed to read from sensor! Code: {}", DHTState::get_state_str(DHTState::get_state_from_code(code)));
                }
            }

            sleep(Duration::from_secs(2));
        }

        assert!(success == 5, "Test Results: {}-{} Success/Fail Ratio", success, 5 - success);

        Ok(())
    }
}