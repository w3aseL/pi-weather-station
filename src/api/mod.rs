use tokio::fs::File;
use std::ffi::OsStr;
use std::io::Read;
use std::path::Path;

use hyper::{Body, Method, Request, Response, StatusCode};
use chrono::{ Local };
use tokio_util::codec::{BytesCodec, FramedRead};
use serde::{ Serialize, Deserialize };

const FORMAT: &'static str = "%Y-%m-%d %H:%M:%S";
const STATIC_LOC: &str = "src/static";

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