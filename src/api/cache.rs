use crate::data::process::{ DataPoint, DaytimeData };
use std::sync::RwLock;
use lazy_static::lazy_static;
use serde::{ Serialize, Deserialize };

struct ApiCache {
	daytime: DaytimeData,
	latest: DataPoint
}

impl ApiCache {
	pub fn new() -> Self {
		Self {
			daytime: DaytimeData::new(None),
			latest: DataPoint::new()
		}
	}

	pub fn update_daytime_data(&mut self, daytime: DaytimeData) {
		self.daytime = daytime;
	}

	pub fn update_latest_data(&mut self, latest: DataPoint) {
		self.latest = latest;
	}

	pub fn get_daytime_data(&self) -> DaytimeData {
		self.daytime.clone()
	}

	pub fn get_latest_data(&self) -> DataPoint {
		self.latest.clone()
	}
}

lazy_static! {
	static ref API_CACHE: RwLock<ApiCache> = RwLock::new(ApiCache::new());
}

pub fn update_api_cache(daytime: Option<DaytimeData>, latest: Option<DataPoint>) {
	let mut cache_update = API_CACHE.write().unwrap();

	if daytime.is_some() {
		cache_update.update_daytime_data(daytime.unwrap());
	}

	if latest.is_some() {
		cache_update.update_latest_data(latest.unwrap());
	}
}

pub fn get_daytime_data() -> DaytimeData {
	let cache_read = API_CACHE.read().unwrap();

	cache_read.get_daytime_data()
}

pub fn get_latest_data() -> DataPoint {
	let cache_read = API_CACHE.read().unwrap();

	cache_read.get_latest_data()
}