use std::time::{ SystemTime };
use crossbeam_channel::{ Sender };

use super::events::{ Payload };
use super::analog::{ MCP3008 };
use crate::data::process::{ DataPoint };

const BUFFER_SIZE: usize = 16;
const INPUT_VOLTAGE: f32 = 3.3;
const OUTPUT_RESISTANCE: u32 = 5100;
const VANE_CALIBRATION_AMOUNT: f32 = 0.0;

const RESISTANCES: [u32; 16] = [
    33000, 6570, 8200, 891,
    1000, 688, 2200, 1410,
    3900, 3140, 16000, 14120,
    120000, 42120, 64900, 21880
];

const DIRECTIONS: [&str; 16] = [
    "N", "NNE", "NE", "ENE",
    "E", "ESE", "SE", "SSE",
    "S", "SSW", "SW", "WSW",
    "W", "WNW", "NW", "NNW"
];

fn find_direction_by_voltage(voltage: f32) -> f32 {
    let mut idx = 0;
    let mut min_diff = 999.0;

    for i in 0..16 {
        let calc_voltage = (INPUT_VOLTAGE * RESISTANCES[i] as f32) / (RESISTANCES[i] + OUTPUT_RESISTANCE) as f32;       // Series resistance formula
        let diff = (voltage - calc_voltage).abs();

        if diff < min_diff {
            idx = i;
            min_diff = diff;
        }
    }

    let mut dir = (idx as f32 * 22.5) + VANE_CALIBRATION_AMOUNT;

    if dir >= 360.0 {
        dir -= 360.0
    }

    dir
}

#[derive(Debug, Clone, Copy)]
pub struct WindVaneData {
    direction: f32,
    last_updated: Option<SystemTime>
}

impl WindVaneData {
    pub fn new(direction: f32, last_updated: Option<SystemTime>) -> Self {
        Self {
            direction: direction,
            last_updated: last_updated
        }
    }

    pub fn is_valid(&self) -> bool {
        self.last_updated.is_some()
    }

    pub fn get_last_updated(&self) -> Option<SystemTime> {
        self.last_updated
    }

    pub fn get_direction(&self) -> f32 {
        self.direction
    }

    pub fn get_dir_as_string(&self) -> String {
        DIRECTIONS[(self.get_direction() / 22.5) as usize].to_string()
    }
}

pub struct WindVanePayload {
    data: WindVaneData
}

impl WindVanePayload {
    pub fn new(direction: f32, last_updated: Option<SystemTime>) -> Self {
        Self {
            data: WindVaneData::new(direction, last_updated)
        }
    }
}

impl Payload for WindVanePayload {
    fn send_message(&self) {
        // ...
    }

    fn update_data_fields(&self, data: &mut DataPoint) {
        data.update_direction(self.data.clone())
    }
}

pub struct WindVane {
    voltage: f32,
    direction: f32,
    mcp: MCP3008,
    mcp_channel: u8,
    payload_sender: Sender<Box<dyn Payload>>,
    buf: [u8; BUFFER_SIZE]
}

impl WindVane {
    pub fn new(mcp: MCP3008, channel: u8, payload_sender: Sender<Box<dyn Payload>>) -> Self {
        Self {
            voltage: 0.0,
            direction: 0.0,
            mcp,
            mcp_channel: channel,
            payload_sender,
            buf: [0u8; BUFFER_SIZE]
        }
    }

    pub fn update_data(&mut self) {
        let bytes_read = self.mcp.read_from_channel(self.mcp_channel, &mut self.buf[..BUFFER_SIZE]);

        self.parse_bits(bytes_read);

        self.payload_sender.send(Box::new(WindVanePayload::new(self.direction, Some(SystemTime::now())))).unwrap();
    }

    fn parse_bits(&mut self, bytes_read: usize) {
        let true_bytes = &self.buf[..bytes_read];
        let mut ret_val = 0u16;
        ret_val |= true_bytes[2] as u16;
        ret_val |= ((true_bytes[1] & 3) as u16) << 8;

        self.voltage = (ret_val as f32 / 0x03FFu16 as f32) * INPUT_VOLTAGE;
        self.direction = find_direction_by_voltage(self.voltage);
    }
}