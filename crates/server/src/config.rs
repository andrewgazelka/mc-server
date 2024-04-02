use std::{fs::File, io::Read, path::Path};

use anyhow::Context;
use serde::{Deserialize, Serialize};
use spin::lazy::Lazy;
use tracing::{info, instrument};

pub static CONFIG: Lazy<Config> = Lazy::new(|| Config::load("run/config.toml").unwrap());

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub border_diameter: Option<f64>,
    pub max_players: i32,
    pub view_distance: i32,
    pub simulation_distance: i32,
    pub server_desc: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            border_diameter: Some(100.0),
            max_players: 10_000,
            view_distance: 32,
            simulation_distance: 10,
            server_desc: "10k babyyyy".to_owned(),
        }
    }
}

impl Config {
    #[instrument(skip_all)]
    pub fn load<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        info!("loading configuration file");
        if path.as_ref().exists() {
            let mut file = File::open(path)?;
            let mut contents = String::default();
            file.read_to_string(&mut contents)?;
            let config = toml::from_str::<Self>(contents.as_str())?;
            Ok(config)
        } else {
            info!("configuration file not found, using defaults");

            // make required folders
            if let Some(parent) = path.as_ref().parent() {
                std::fs::create_dir_all(parent).with_context(|| {
                    format!(
                        "failed to create parent directories for {:?}",
                        path.as_ref()
                    )
                })?;
            }

            // write default config to file
            let default_config = Self::default();
            std::fs::write(&path, toml::to_string(&default_config)?.as_bytes())?;
            
            info!("wrote default configuration to {:?}", path.as_ref());

            Ok(Self::default())
        }
    }
}
