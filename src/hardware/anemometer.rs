use rppal::gpio::{ InputPin, Trigger };
use std::time::{ SystemTime };
use crossbeam_channel::{ Sender };

use super::events::{ Event, Payload, EventType };
use crate::data::process::{ DataPoint, DaytimeData };

const CM_TO_KM: f32 = 100000.0;
const SEC_TO_HR: f32 = 3600.0;
const KM_TO_MI: f32 = 1.609344;
const WIND_ADJUSTMENT: f32 = 1.18;

#[derive(Debug, Clone, Copy)]
pub struct AnemometerData {
    spins_per_sec: f32,
    last_updated: Option<SystemTime>
}

impl AnemometerData {
    pub fn new(spins_per_sec: f32, last_updated: Option<SystemTime>) -> Self {
        Self {
            spins_per_sec,
            last_updated
        }
    }

    pub fn is_valid(&self) -> bool {
        self.last_updated.is_some()
    }

    pub fn get_last_updated(&self) -> Option<SystemTime> {
        self.last_updated
    }

    fn get_cm_per_sec(&self) -> f32 {
        (self.spins_per_sec / 2.0) * ((2.0 * std::f32::consts::PI) * 9.0)
    }

    pub fn get_kph(&self) -> f32{
        (self.get_cm_per_sec() / CM_TO_KM) * SEC_TO_HR * WIND_ADJUSTMENT
    }

    pub fn get_mph(&self) -> f32 {
        self.get_kph() / KM_TO_MI
    }

    pub fn convert_to_kph(spins: f32) -> f32 {
        (spins / 2.0) * ((2.0 * std::f32::consts::PI) * 9.0)
    }

    pub fn convert_to_mph(spins: f32) -> f32 {
        Self::convert_to_kph(spins) / KM_TO_MI
    }
}

pub struct AnemometerPayload {
    data: AnemometerData
}

impl AnemometerPayload {
    pub fn new(spins_per_sec: f32, last_updated: Option<SystemTime>) -> Self {
        Self {
            data: AnemometerData::new(spins_per_sec, last_updated)
        }
    }
}

impl Payload for AnemometerPayload {
    fn send_message(&self) {
        // ...
    }

    fn update_data_fields(&self, data: &mut DataPoint, daytime_info: &mut DaytimeData) {
        if daytime_info.wind_min < 0.0 || daytime_info.wind_min > self.data.spins_per_sec {
            daytime_info.wind_min = self.data.spins_per_sec;
        }

        if daytime_info.wind_max < self.data.spins_per_sec {
            daytime_info.wind_max = self.data.spins_per_sec;
        }

        data.update_anemometer(self.data.clone());
    }
}

pub struct Anemometer {
    pin: InputPin,
    sender: Sender<Event>,
    payload_sender: Sender<Box<dyn Payload>>,
    counter: i32,
    spins_per_sec: f32,
    last_updated: SystemTime
}

impl Anemometer {
    pub fn new(pin: InputPin, sender: Sender<Event>, payload_sender: Sender<Box<dyn Payload>>) -> Self {
        Self {
            pin,
            sender,
            payload_sender,
            counter: 0,
            spins_per_sec: 0.0,
            last_updated: SystemTime::now()
        }
    }

    pub fn start(&mut self) {
        let copy_sender = self.sender.clone();

        self.pin.set_async_interrupt(Trigger::RisingEdge, move |_| {
            copy_sender.send(Event::new(EventType::AnemometerCount)).unwrap();
        }).unwrap();
    }

    pub fn increment_counter(&mut self) {
        self.counter += 1;
    }

    pub fn update_data(&mut self) {
        let time_elapsed = self.last_updated.elapsed().unwrap().as_millis();

        self.spins_per_sec = self.counter as f32 / (time_elapsed as f32 / 1000.0);
        self.counter = 0;
        self.last_updated = SystemTime::now();

        self.payload_sender.send(Box::new(AnemometerPayload::new(self.spins_per_sec, Some(self.last_updated)))).unwrap();
    }
}