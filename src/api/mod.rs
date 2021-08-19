pub mod cache;

use serde_json::json;
use tokio::fs::File;
use std::ffi::OsStr;
use std::io::Read;
use std::time::SystemTime;
use std::path::Path;

use hyper::{Body, Method, Request, Response, StatusCode};
use chrono::{ DateTime, Local };
use tokio_util::codec::{BytesCodec, FramedRead};
use serde::{ Serialize, Deserialize };
use serde_json::{ Value };

use cache::{ get_daytime_data, get_latest_data };

const FORMAT: &'static str = "%Y-%m-%d %H:%M:%S";
const STATIC_LOC: &str = "static";

fn get_404_res() -> Response<Body> {
	Response::builder()
		.status(StatusCode::NOT_FOUND)
		.body("Not Found!".into())
		.unwrap()
}

async fn get_static_file(path: &str) -> Response<Body> {
	let body = match File::open(STATIC_LOC.to_owned() + path).await {
        Ok(f) => {
			let stream = FramedRead::new(f, BytesCodec::new());
    		Body::wrap_stream(stream)
		},
        Err(_) => {
            return get_404_res();
        },
    };

	const UNKNOWN_CONTENT_TYPE: &str = "text/plain";
	let content_type = match Path::new(path).extension().and_then(OsStr::to_str) {
		Some(ext) => match ext {
			"html" => "text/html",
			"css" => "text/css",
			"js" => "application/javascript",
			_ => UNKNOWN_CONTENT_TYPE,
		},
		None => UNKNOWN_CONTENT_TYPE,
	};

	Response::builder()
		.header("Content-Type", content_type)
		.body(body)
		.unwrap()
}

fn get_local_time_from_system_time(time: SystemTime) -> String {
	let dt: DateTime<Local> = time.into();

	dt.format("%FT%T%z").to_string()
}

#[derive(Serialize, Deserialize)]
struct ApiInfo {
	version: String
}

async fn get_api_data(method: &Method, path: &str) -> Response<Body> {
	match (method, path) {
		(_, "") | (_, "/") => {
			let string = serde_json::to_string(&ApiInfo {
				version: String::from("v0.1.0")
			}).expect("Failed to parse to JSON!");

			Response::builder()
				.header("Content-Type", "application/json")
				.body(string.into())
				.unwrap()
		},
		(&Method::GET, "/latest") => {
			let data = get_latest_data();

			// wind speed
			let wind_spd = data.get_anemometer_data();
			let wind_spd_data = if wind_spd.is_valid() { json!({
				"mph": wind_spd.get_mph(),
				"kph": wind_spd.get_kph(),
				"last_updated": get_local_time_from_system_time(wind_spd.get_last_updated().unwrap())
			}) } else { json!(null) };

			// wind dir
			let wind_dir = data.get_directional_data();
			let wind_dir_data = if wind_dir.is_valid() { json!({
				"dir": wind_dir.get_direction(),
				"label": wind_dir.get_dir_as_string(),
				"last_updated": get_local_time_from_system_time(wind_dir.get_last_updated().unwrap())
			}) } else { json!(null) };

			// instantaneous rain data
			let rain = data.get_rain_data();
			let rain_data = if rain.is_valid() { json!({
				"amnt_in": rain.get_amount_in(),
				"amnt_cm": rain.get_amount_cm(),
				"last_updated": get_local_time_from_system_time(rain.get_last_updated().unwrap())
			}) } else { json!(null) };

			// temp/humidity
			let temp = data.get_temp_data();
			let temp_data = if temp.is_valid() { json!({
				"temp_f": temp.get_temp_farenheit(),
				"temp_c": temp.get_temp_celsius(),
				"humidity": temp.get_humidity(),
				"last_updated": get_local_time_from_system_time(temp.get_last_updated().unwrap())
			})} else { json!(null) };

			//json obj
			let json_data = json!({
				"wind": wind_spd_data,
				"wind_dir": wind_dir_data,
				"temp": temp_data,
				"rain": rain_data
			});

			Response::builder()
				.header("Content-Type", "application/json")
				.body(json_data.to_string().into())
				.unwrap()
		},
		_ => {
			get_404_res()
		}
	}
}

pub async fn api_service(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let res = match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => get_static_file("/index.html").await,
        (method, path) => {
			if method == &Method::GET && Path::new(path).extension().is_some() {
				get_static_file(path).await
			} else {
				match &path[..4] {
					"/api" => {
						get_api_data(method, &path[4..]).await
					},
					_ => get_404_res()
				}
			}
        }
    };

	println!("[{}] {} \"{}\" -- {}", Local::now().format(FORMAT), req.method(), req.uri().path(), res.status().as_u16());

	Ok(res)
}