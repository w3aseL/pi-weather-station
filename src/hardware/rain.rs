use rppal::gpio::{ InputPin, Trigger };
use std::time::{ SystemTime };
use crossbeam_channel::{ Sender };

use super::events::{ Payload, Event, EventType };
use crate::data::process::{ DataPoint };

const COUNT_TO_MM: f32 = 0.2794;
const CM_TO_MM: f32 = 10.0;
const CM_TO_IN: f32 = 2.54;

#[derive(Debug, Clone, Copy)]
pub struct RainData {
    ticks_per_sec: f32,
    last_updated: Option<SystemTime>
}

impl RainData {
    pub fn new(ticks_per_sec: f32, last_updated: Option<SystemTime>) -> Self {
        Self {
            ticks_per_sec,
            last_updated
        }
    }

    pub fn is_valid(&self) -> bool {
        self.last_updated.is_some()
    }

    pub fn get_last_updated(&self) -> Option<SystemTime> {
        self.last_updated
    }

    pub fn get_amount_cm(&self) -> f32 {
        (self.ticks_per_sec / COUNT_TO_MM) / CM_TO_MM
    }

    pub fn get_amount_in(&self) -> f32 {
        self.get_amount_cm() / CM_TO_IN
    }
}

pub struct RainPayload {
    data: RainData
}

impl RainPayload {
    pub fn new(ticks_per_sec: f32, last_updated: Option<SystemTime>) -> Self {
        Self {
            data: RainData::new(ticks_per_sec, last_updated)
        }
    }
}

impl Payload for RainPayload {
    fn send_message(&self) {
        // ...
    }

    fn update_data_fields(&self, data: &mut DataPoint) {
        data.update_rain(self.data.clone());
    }
}

pub struct RainMeter {
    pin: InputPin,
    sender: Sender<Event>,
    payload_sender: Sender<Box<dyn Payload>>,
    counter: i32,
    ticks_per_sec: f32,
    last_updated: SystemTime
}

impl RainMeter {
    pub fn new(pin: InputPin, sender: Sender<Event>, payload_sender: Sender<Box<dyn Payload>>) -> Self {
        Self {
            pin,
            sender,
            payload_sender,
            counter: 0,
            ticks_per_sec: 0.0,
            last_updated: SystemTime::now()
        }
    }

    pub fn start(&mut self) {
        let copy_sender = self.sender.clone();

        self.pin.set_async_interrupt(Trigger::RisingEdge, move |_| {
            copy_sender.send(Event::new(EventType::RainCount)).unwrap();
        }).unwrap();
    }

    pub fn increment_counter(&mut self) {
        self.counter += 1;
    }

    pub fn update_data(&mut self) {
        let time_elapsed = self.last_updated.elapsed().unwrap().as_millis();

        self.ticks_per_sec = self.counter as f32 / (time_elapsed as f32 / 1000.0);
        self.counter = 0;
        self.last_updated = SystemTime::now();

        self.payload_sender.send(Box::new(RainPayload::new(self.ticks_per_sec, Some(self.last_updated)))).unwrap();
    }
}