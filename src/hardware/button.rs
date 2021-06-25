use rppal::gpio::{ InputPin, Trigger, Level };
use std::time::{ SystemTime };
use crossbeam_channel::{ Sender };
use chrono::{ DateTime };
use chrono::offset::{ Utc };

use super::events::{ Event, EventType, Payload };
use crate::data::process::{ DataPoint };

#[derive(Debug, Clone, Copy)]
pub struct IncrementalPayload {
    presses_per_sec: f32,
    last_updated: SystemTime
}

impl IncrementalPayload {
    pub fn new(presses_per_sec: f32, last_updated: SystemTime) -> Self {
        Self {
            presses_per_sec,
            last_updated
        }
    }
}

impl Payload for IncrementalPayload {
    fn send_message(&self) {
        let time: DateTime<Utc> = self.last_updated.into();

        println!("Received Button Payload --- {} CPS, Last Updated: {}", self.presses_per_sec, time.format("%d/%m/%Y %T").to_string());
    }

    fn update_data_fields(&self, data: &mut DataPoint) {
        data.update_message(format!("Received Button Payload --- {} CPS", self.presses_per_sec).to_string());
    }
}

pub struct Button {
    pin: InputPin,
    sender: Sender<Event>,
    payload_sender: Sender<Box<dyn Payload>>,
    counter: i32,
    presses_per_sec: f32,
    last_updated: SystemTime
}

impl Button {
    pub fn new(pin: InputPin, sender: Sender<Event>, payload_sender: Sender<Box<dyn Payload>>) -> Self {
        Self {
            pin,
            sender,
            payload_sender,
            counter: 0,
            presses_per_sec: 0.0,
            last_updated: SystemTime::now()
        }
    }

    pub fn start(&mut self) {
        /*
        let copy_sender = self.sender.clone();

        self.pin.set_async_interrupt(Trigger::Both, move |_| {
            let event_type = if level == Level::Low { EventType::ButtonRelease } else { EventType::ButtonPress };

            copy_sender.send(Event::new(event_type)).unwrap();
        }).unwrap();
        */
    }

    pub fn increment_counter(&mut self) {
        self.counter += 1;
    }

    pub fn update_data(&mut self) {
        let time_elapsed = self.last_updated.elapsed().unwrap().as_millis();

        self.presses_per_sec = self.counter as f32 / (time_elapsed as f32 / 1000.0);
        self.counter = 0;
        self.last_updated = SystemTime::now();

        self.payload_sender.send(Box::new(IncrementalPayload::new(self.presses_per_sec, self.last_updated))).unwrap();
    }
}