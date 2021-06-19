use rppal::gpio::{ InputPin, Trigger, Level };
use std::time::{ SystemTime };
use crossbeam_channel::{ Sender };
use chrono::{ DateTime };
use chrono::offset::{ Utc };

use super::events::{ Event, EventType, Payload };
use super::button::{ IncrementalPayload };

pub struct Anemometer {
    pin: InputPin,
    sender: Sender<Event>,
    payload_sender: Sender<Box<dyn Payload>>,
    counter: i32,
    presses_per_sec: f32,
    last_updated: SystemTime
}

impl Anemometer {
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
        let copy_sender = self.sender.clone();

        self.pin.set_async_interrupt(Trigger::RisingEdge, move |level| {
            println!("Anemometer detected on rising edge: {:?}", level);

            // copy_sender.send(Event::new(EventType::AnemometerCount)).unwrap();
        }).unwrap();

        self.pin.set_async_interrupt(Trigger::FallingEdge, move |level| {
            println!("Anemometer detected on falling edge: {:?}", level);

            // copy_sender.send(Event::new(EventType::AnemometerCount)).unwrap();
        }).unwrap();
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