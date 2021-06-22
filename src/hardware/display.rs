use std::error::Error;
use std::time::{ Duration };
use std::thread::{ sleep };

use rppal::gpio::{ Gpio, IoPin, Mode, Level };

// Commands
const CLEAR_DISPLAY: u8 = 0x01;
const RETURN_HOME: u8 = 0x02;
const ENTRY_MODE_SET: u8 = 0x04;
const DISPLAY_CONTROL: u8 = 0x08;
const CURSOR_SHIFT: u8 = 0x10;
const FUNCTION_SET: u8 = 0x20;
const SET_CGRAM_ADDR: u8 = 0x40;
const SET_DDRAM_ADDR: u8 = 0x80;

// Entry flags
const ENTRY_RIGHT: u8 = 0x00;
const ENTRY_LEFT: u8 = 0x02;
const ENTRY_SHIFT_INCREMENT: u8 = 0x01;
const ENTRY_SHIFT_DECREMENT: u8 = 0x00;

// Control flags
const DISPLAY_ON: u8 = 0x04;
const DISPLAY_OFF: u8 = 0x00;
const CURSOR_ON: u8 = 0x02;
const CURSOR_OFF: u8 = 0x00;
const BLINK_ON: u8 = 0x01;
const BLINK_OFF: u8 = 0x00;

// Move flags
const DISPLAY_MOVE: u8 = 0x08;
const CURSOR_MOVE: u8 = 0x00;
const MOVE_RIGHT: u8 = 0x04;
const MOVE_LEFT: u8 = 0x00;

// Function set flags
const EIGHT_BIT_MODE: u8 = 0x10;
const FOUR_BIT_MODE: u8 = 0x00;
const TWO_LINE: u8 = 0x08;
const ONE_LINE: u8 = 0x00;
const FIVE_BY_TEN_DOTS: u8 = 0x04;
const FIVE_BY_EIGHT_DOTS: u8 = 0x00;

// Row offsets
const ROW_OFFSETS: [u8; 4] = [ 0x00, 0x40, 0x14, 0x54 ];

pub struct LCDDisplay {
    rs: IoPin,
    en: IoPin,
    d_pins: Vec<IoPin>,
    cols: usize,
    rows: usize,
    display_control: u8,
    display_function: u8,
    display_mode: u8
}

impl LCDDisplay {
    pub fn new(rs_pin: u8, en_pin: u8, d_pins: [u8; 4], cols: usize, rows: usize) -> Result<Self, Box<dyn Error>> {
        let rs = Gpio::new()?.get(rs_pin)?.into_io(Mode::Output);
        let en = Gpio::new()?.get(en_pin)?.into_io(Mode::Output);
        let mut d_pins_vec = Vec::new();

        for i in 0..4 {
            let io_pin = Gpio::new()?.get(d_pins[i])?.into_io(Mode::Output);

            d_pins_vec.push(io_pin);
        }

        let mut display = Self {
            rs,
            en,
            d_pins: d_pins_vec,
            cols,
            rows,
            display_control: DISPLAY_ON | CURSOR_OFF | BLINK_OFF,
            display_function: FOUR_BIT_MODE | ONE_LINE | TWO_LINE | FIVE_BY_EIGHT_DOTS,
            display_mode: ENTRY_LEFT | ENTRY_SHIFT_DECREMENT
        };

        // Initialize the display
        display.write_bit(0x33, false);
        display.write_bit(0x32, false);

        display.write_bit(DISPLAY_CONTROL | display.display_control, false);
        display.write_bit(FUNCTION_SET | display.display_function, false);
        display.write_bit(ENTRY_MODE_SET | display.display_mode, false);

        display.clear();

        Ok(display)
    }

    pub fn clear(&mut self) {
        self.write_bit(CLEAR_DISPLAY, false);
        sleep(Duration::from_micros(3000));
    }

    pub fn cursor_home(&mut self) {
        self.write_bit(RETURN_HOME, false);
        sleep(Duration::from_micros(3000));
    }

    fn set_cursor(&mut self, col: usize, row: usize) {
        if row > self.rows {
            self.write_bit(SET_DDRAM_ADDR | (col as u8 + ROW_OFFSETS[self.rows - 1]), false);
        } else {
            self.write_bit(SET_DDRAM_ADDR | (col as u8 + ROW_OFFSETS[row]), false);
        }
    }

    pub fn write_message(&mut self, message: String) {
        let mut line = 0;

        if !message.is_ascii() {
            return;
        }

        for char in message.chars() {
            if char == '\n' {
                line += 1;

                let col = if self.display_mode & ENTRY_LEFT > 0 { 0 } else { self.cols - 1 };

                self.set_cursor(col, line);
            } else {
                self.write_bit(char as u8, true);
            }
        }
    }

    fn write_bit(&mut self, value: u8, char_mode: bool) {
        sleep(Duration::from_micros(1000));

        self.rs.write(if char_mode { Level::High } else { Level::Low });

        // Write upper bits
        for i in 0..4 {
            if ((value >> (i+4)) & 1) > 0 {
                self.d_pins[i].set_high();
            } else {
                self.d_pins[i].set_low();
            }
        }

        self.pulse_enable();

        // Write lower bits
        for i in 0..4 {
            let shift = if i > 0 { value >> i } else { value };                 // don't shift when first bit

            if (shift & 1) > 0 {
                self.d_pins[i].set_high();
            } else {
                self.d_pins[i].set_low()
            }
        }

        self.pulse_enable();
    }

    fn pulse_enable(&mut self) {
        self.en.set_low();
        sleep(Duration::from_micros(1));
        self.en.set_high();
        sleep(Duration::from_micros(1));
        self.en.set_low();
        sleep(Duration::from_micros(1));
    }
}