use std::iter::Empty;

#[derive(Debug, Clone, Copy)]
pub enum EventType {
    SensorRead,
    ButtonPress,
    ButtonRelease,
    UpdateData,
    ReceiveData,
    AnemometerCount,
    Exit
}

#[derive(Debug, Clone, Copy)]
pub struct Event {
    event_type: EventType
}

impl Event {
    pub fn new(event_type: EventType) -> Self {
        Self {
            event_type
        }
    }

    pub fn get_event_type(&self) -> EventType {
        self.event_type
    }
}

pub trait Payload: Send {
    fn send_message(&self);
}

#[derive(Debug, Clone, Copy)]
pub struct EmptyPayload {}

impl EmptyPayload {
    pub fn new() -> Self {
        Self {}
    }
}

impl Payload for EmptyPayload {
    fn send_message(&self) {
        println!("Printing empty payload!");
    }
}