use crate::hardware::events::{ Event, EventType, Payload };
use crossbeam_channel::{ Sender, Receiver };
use std::thread::{ sleep, spawn };
use std::time::{ Duration };

pub struct DataManager {
    sender: Sender<Event>,
    receiver: Receiver<Box<dyn Payload>>
}

impl DataManager {
    pub fn new(sender: Sender<Event>, receiver: Receiver<Box<dyn Payload>>) -> Self {
        Self {
            sender,
            receiver
        }
    }

    pub fn start(mut self) {
        spawn(move || {
            loop {
                self.sender.send(Event::new(EventType::UpdateData)).unwrap();

                sleep(Duration::from_millis(5));

                while let Ok(payload) = self.receiver.try_recv() {
                    payload.send_message();
                }

                sleep(Duration::from_millis(995));
            }
        });
    }
}