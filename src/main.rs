extern crate rppal;
extern crate chrono;
extern crate crossbeam_channel;

mod hardware;
mod data;

use hardware::dht::{ DHT, DHTSensor };
use hardware::button::{ Button };
use hardware::events::{ EventType };
use hardware::anemometer::{ Anemometer };

use data::process::{ DataManager };

use std::error::Error;
use std::time::{ Duration };
use std::thread::{ sleep };

use rppal::gpio::{ Gpio, Mode };
use crossbeam_channel as channel;

fn main() -> Result<(), Box<dyn Error>> {
    let (tx, rx) = channel::unbounded();
    let (payload_tx, payload_rx) = channel::unbounded();

    let dht_sensor = DHT::new(Gpio::new()?.get(18)?.into_io(Mode::Input), DHTSensor::DHT11, tx.clone(), payload_tx.clone());
    
    dht_sensor.start_reading();

    println!("Starting button async task!");

    let mut button = Button::new(Gpio::new()?.get(23)?.into_input(), tx.clone(), payload_tx.clone());

    button.start();

    println!("Starting LED output pin!");

    let mut led_pin = Gpio::new()?.get(15)?.into_output();
    let mut button_led = Gpio::new()?.get(24)?.into_output();

    let mut anemometer = Anemometer::new(Gpio::new()?.get(8)?.into_input(), tx.clone(), payload_tx.clone());

    anemometer.start();

    let manager = DataManager::new(tx.clone(), payload_rx.clone());

    manager.start();

    loop { 
        if let Ok(event) = rx.try_recv() {
            match event.get_event_type() {
                EventType::SensorRead => {
                    led_pin.set_high();
                    sleep(Duration::from_millis(100));
                    led_pin.set_low();
                },
                EventType::ButtonPress => {
                    button.increment_counter();
                    button_led.set_high();
                    sleep(Duration::from_millis(20));
                },
                EventType::ButtonRelease => {
                    button_led.set_low();
                    sleep(Duration::from_millis(20));
                },
                EventType::AnemometerCount => {
                    // anemometer.increment_counter();
                },
                EventType::UpdateData => {
                    button.update_data();
                    // anemometer.update_data();
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