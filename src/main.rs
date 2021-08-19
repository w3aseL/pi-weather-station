extern crate rppal;
extern crate chrono;
extern crate crossbeam_channel;
extern crate sysinfo;
extern crate toml;
extern crate postgres;
extern crate reqwest;
extern crate tokio;
extern crate hyper;
extern crate job_scheduler;
extern crate lazy_static;

mod config;
mod db;
mod hardware;
mod data;
mod api;

use hardware::dht::DHT;
//use hardware::button::{ Button };
use hardware::events::{ Event, EventType };
use hardware::anemometer::{ Anemometer };
use hardware::analog::{ MCP3008 };
use hardware::vane::{ WindVane };
use hardware::rain::{ RainMeter };

use data::process::{ DataManager };

use config::{ Config };

use api::{ api_service };

use std::error::Error;
use std::net::{ SocketAddr };
use std::thread::{ sleep };
use std::time::{ Duration };

use rppal::gpio::{ Gpio, Mode };
use rppal::spi::{ Bus, SlaveSelect, Mode as SPIMode };
use job_scheduler::{ JobScheduler, Job };
use crossbeam_channel as channel;
use hyper::service::{ make_service_fn, service_fn };
use hyper::{ Server };
use lazy_static::lazy_static;

lazy_static! {
    static ref CONFIG: Config = Config::retrieve_config();
}

const DHT_PIN: u8 = 4;
const ANEMOMETER_PIN: u8 = 5;
const RAIN_PIN: u8 = 6;

#[tokio::main]
async fn tokio_main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let socket_addr = format!("0.0.0.0:{}", if CONFIG.is_prod_env() { 8080u16 } else { 3000u16 }).parse::<SocketAddr>().unwrap();

    let service = make_service_fn(|_| async { Ok::<_, hyper::Error>(service_fn(api_service)) }); 
    
    let server = Server::bind(&socket_addr).serve(service);

    server.await?;

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    // Channels for messaging. Currently used as MPSC
    let (tx, rx) = channel::unbounded();
    let (payload_tx, payload_rx) = channel::unbounded();

    // Humidity and Temperature Init
    let mut dht_sensor = DHT::new(Gpio::new()?.get(DHT_PIN)?.into_io(Mode::Input), tx.clone(), payload_tx.clone());

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
    let (time_tx, time_rx) = channel::unbounded();
    let manager = DataManager::new(tx.clone(), payload_rx.clone(), time_rx.clone(), CONFIG.clone())?;
    manager.start();

    std::thread::spawn(move || {
        let res = tokio_main();
    });

    let mut schedule = JobScheduler::new();

    // Rain job
    let rain_job_sender = tx.clone();
    schedule.add(Job::new("0 0/5 * * * *".parse().unwrap(), move || {
        rain_job_sender.send(Event::new(EventType::UpdateRain)).unwrap();
    }));

    // Wind job
    let wind_job_sender = tx.clone();
    schedule.add(Job::new("0 0/5 * * * *".parse().unwrap(), move || {
        wind_job_sender.send(Event::new(EventType::UpdateWind)).unwrap();
    }));

    // Temp job
    let temp_job_sender = tx.clone();
    schedule.add(Job::new("0 0/1 * * * *".parse().unwrap(), move || {
        temp_job_sender.send(Event::new(EventType::UpdateTemp)).unwrap();
    }));

    // Data refresh job
    let data_refresh_sender = time_tx.clone();
    schedule.add(Job::new("0 0 0 * * *".parse().unwrap(), move || {
        data_refresh_sender.send(Event::new(EventType::MidnightRefresh)).unwrap();
    }));

    loop {
        schedule.tick();

        if let Ok(event) = rx.try_recv() {
            match event.get_event_type() {
                EventType::AnemometerCount => {
                    anemometer.increment_counter();
                },
                EventType::RainCount => {
                    rain_guage.increment_counter();
                },
                EventType::UpdateData => {
                    // button.update_data();
                    // anemometer.update_data();
                    // rain_guage.update_data();
                    // wind_vane.update_data();
                },
                EventType::UpdateRain => {
                    rain_guage.update_data();
                },
                EventType::UpdateWind => {
                    anemometer.update_data();
                    wind_vane.update_data();
                },
                EventType::UpdateTemp => {
                    dht_sensor.update_data();
                },
                EventType::Exit => {
                    println!("Exiting program!");
                    break;
                },
                _ => {}
            }
        }

        sleep(Duration::from_millis(100));
    }

    Ok(())
}