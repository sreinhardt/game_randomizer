use std::fs;
use serde::{Serialize, Deserialize};

#[derive(Deserialize)]
pub struct Config {
    pub discord: String,
    pub steam: String,
    pub storage: String
}
impl Config {
    pub fn from_file(path: &str) -> Result<Self, ()> {
        let data = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(_) => return Err(())
        };
        let config: Config = match toml::from_str(&data) {
            Ok(c) => c,
            Err(_) => return Err(())
        };
        Ok(config)
    }
}
