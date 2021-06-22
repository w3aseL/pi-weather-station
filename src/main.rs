extern crate rppal;
extern crate chrono;
extern crate crossbeam_channel;

mod hardware;
mod data;

use hardware::dht::{ DHT, DHTSensor };
//use hardware::button::{ Button };
use hardware::events::{ EventType };
use hardware::anemometer::{ Anemometer };
use hardware::analog::{ MCP3008 };
use hardware::vane::{ WindVane };
use hardware::rain::{ RainMeter };

use data::process::{ DataManager };

use std::error::Error;
use std::time::{ Duration };
use std::thread::{ sleep };

use rppal::gpio::{ Gpio, Mode };
use rppal::spi::{ Bus, SlaveSelect, Mode as SPIMode };
use crossbeam_channel as channel;

const DHT_PIN: u8 = 14;
const ANEMOMETER_PIN: u8 = 5;
const RAIN_PIN: u8 = 6;

fn main() -> Result<(), Box<dyn Error>> {
    // Channels for messaging. Currently used as MPSC
    let (tx, rx) = channel::unbounded();
    let (payload_tx, payload_rx) = channel::unbounded();

    // Humidity and Temperature Init
    let dht_sensor = DHT::new(Gpio::new()?.get(DHT_PIN)?.into_io(Mode::Input), DHTSensor::DHT11, tx.clone(), payload_tx.clone());
    dht_sensor.start_reading();

    // let mut button = Button::new(Gpio::new()?.get(23)?.into_input(), tx.clone(), payload_tx.clone());

    // let mut led_pin = Gpio::new()?.get(15)?.into_output();
    // let mut button_led = Gpio::new()?.get(24)?.into_output();

    // Anemometer Init
    let mut anemometer = Anemometer::new(Gpio::new()?.get(ANEMOMETER_PIN)?.into_input_pullup(), tx.clone(), payload_tx.clone());
    anemometer.start();

    // Wind vane init
    let mut wind_vane = WindVane::new(MCP3008::new(Bus::Spi0, SlaveSelect::Ss0, 1000000u32, SPIMode::Mode0), 0, payload_tx.clone());

    // Rain guage init
    let mut rain_guage = RainMeter::new(Gpio::new()?.get(RAIN_PIN)?.into_input_pullup(), tx.clone(), payload_tx.clone());
    rain_guage.start();

    // Data Manager init
    let manager = DataManager::new(tx.clone(), payload_rx.clone())?;
    manager.start();

    loop { 
        if let Ok(event) = rx.try_recv() {
            match event.get_event_type() {
                EventType::SensorRead => {
                    // led_pin.set_high();
                    sleep(Duration::from_millis(100));
                    // led_pin.set_low();
                },
                EventType::ButtonPress => {
                    // button.increment_counter();
                    // button_led.set_high();
                    sleep(Duration::from_millis(20));
                },
                EventType::ButtonRelease => {
                    // button_led.set_low();
                    sleep(Duration::from_millis(20));
                },
                EventType::AnemometerCount => {
                    anemometer.increment_counter();
                },
                EventType::RainCount => {
                    rain_guage.increment_counter();
                },
                EventType::UpdateData => {
                    // button.update_data();
                    anemometer.update_data();
                    rain_guage.update_data();
                    wind_vane.update_data();
                },
                EventType::Exit => {
                    println!("Exiting program!");
                    break;
                },
                _ => {  }
            }
        }
    }

    Ok(())
}