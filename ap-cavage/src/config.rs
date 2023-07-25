use std::fs;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Key {
    pub id: String,
    pub actor: String,
    pub pem: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub key: Vec<Key>,
}

impl Config {
    pub fn from_file(path: &str) -> Self {
        let cfg = fs::read_to_string(path).expect("Unable to read file");
        let config: Self = toml::from_str(&cfg).expect("Invalid config");

        config
    }
}
