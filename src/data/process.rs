use crossbeam_channel::{ Sender, Receiver };
use std::thread::{ sleep, spawn };
use std::time::{ Duration, SystemTime };
use chrono::{ DateTime };
use chrono::offset::{ Utc };

use crate::hardware::events::{ Event, EventType, Payload };
use crate::hardware::dht::{ DHTData };
use crate::hardware::anemometer::{ AnemometerData };
use crate::hardware::vane::{ WindVaneData };

pub struct DataPoint {
    message: Option<String>,
    last_updated: Option<SystemTime>,
    dht_data: DHTData,
    anemometer_data: AnemometerData,
    directional_data: WindVaneData
}

impl DataPoint { 
    pub fn new() -> Self {
        Self {
            message: None,
            last_updated: None,
            dht_data: DHTData::new(-999.0, -999.0, None),
            anemometer_data: AnemometerData::new(0.0, None),
            directional_data: WindVaneData::new(0.0, None)
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

        if data_str.len() > 0 { print!("{}", data_str); }
    }
}

pub struct DataManager {
    sender: Sender<Event>,
    receiver: Receiver<Box<dyn Payload>>,
    data: DataPoint
}

impl DataManager {
    pub fn new(sender: Sender<Event>, receiver: Receiver<Box<dyn Payload>>) -> Self {
        Self {
            sender,
            receiver,
            data: DataPoint::new()
        }
    }

    pub fn start(mut self) {
        spawn(move || {
            loop {
                self.sender.send(Event::new(EventType::UpdateData)).unwrap();

                sleep(Duration::from_millis(5));

                while let Ok(payload) = self.receiver.try_recv() {
                    payload.update_data_fields(&mut self.data);
                }

                sleep(Duration::from_millis(5));

                self.data.print_data();

                sleep(Duration::from_millis(990));
            }
        });
    }
}