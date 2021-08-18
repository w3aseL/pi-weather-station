use serde::{ Deserialize };
use std::fs::read_to_string;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub env: String,
    pub dev: Dev,
    pub prod: Prod
}

#[derive(Deserialize, Debug, Clone)]
pub struct Dev {
    pub addr: String,
    pub username: String,
    pub password: String,
    pub dbname: String
}

#[derive(Deserialize, Debug, Clone)]
pub struct Prod {
    pub addr: String,
    pub username: String,
    pub password: String,
    pub dbname: String
}

impl Config {
    pub fn retrieve_config() -> Self {
        let config_str = read_to_string("Config.toml").expect("Failed to open Config.toml");

        let config: Config = toml::from_str(&config_str).unwrap();

        config
    }

    pub fn is_prod_env(&self) -> bool {
        "prod" == &self.env
    }
}