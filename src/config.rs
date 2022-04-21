use std::fs;
use std::fs::File;
use std::path::Path;
use serenity::prelude::*;
use serde::Deserialize;

use crate::commands::{
    suggestions::GameSuggestions,
    players::PlayerContainer
};

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
    pub fn load_suggestions(&self) -> Option<<GameSuggestions as TypeMapKey>::Value> {
        let storage = Path::new(&self.storage);
        match File::open(storage.join("suggestions.json")) {
            Err(_) => {
                println!{"Failure opening suggestions file"};
                return None;
            },
            Ok(p) => {
                match serde_json::from_reader(p) {
                    Err(_) => {
                        println!{"Failure deserializing suggestions file"};
                        return None;
                    },
                    Ok(s) => {
                        println!{"Opened previous suggestions file!"}
                        Some(s)
                    }
                }
            }
        }
    }
    pub fn load_players(&self) -> Option<<PlayerContainer as TypeMapKey>::Value> {
        let storage = Path::new(&self.storage);
        match File::open(storage.join("players.json")) {
            Err(_) => {
                println!{"Failure opening players file"};
                return None;
            },
            Ok(p) => {
                match serde_json::from_reader(p) {
                    Err(_) => {
                        println!{"Failure deserializing players file"};
                        return None;
                    },
                    Ok(s) => {
                        println!{"Opened previous players file!"}
                        Some(s)
                    }
                }
            }
        }
    }
}
impl TypeMapKey for Config {
    type Value = Config;
}
