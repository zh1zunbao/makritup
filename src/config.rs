//! Global configuration management
//! This module handles the loading and parsing of configuration files
//! and environment variables for the application.
//! Usage:
//! ```rust
//! use markitup::config::SETTINGS;
//! // fn main() {
//! //     let cfg = &SETTINGS;
//! //     println!("{:?}", cfg.model_path);
//! // }

use config::{Config, ConfigError, Environment, File, FileFormat};
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::{env, path::PathBuf, fs};

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub model_path: PathBuf,
    pub image_path: PathBuf,
    pub is_ai_enpower: bool,
    pub doubao_api_key: Option<String>,
}


pub static SETTINGS: Lazy<Settings> = Lazy::new(|| {
    Settings::new().unwrap_or_else(|e| {
        eprintln!("Failed to load configuration: {}", e);
        std::process::exit(1);
    })
});


impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        // 1. built-in default config
        let mut builder = Config::builder()
            .add_source(File::from_str(
                include_str!("../Config.toml"),
                FileFormat::Toml,
            ));

        // 2. try to load external config file
        if let Ok(exe_path) = env::current_exe() {
            if let Some(dir) = exe_path.parent() {
                let external = dir.join("Config.toml");
                if fs::metadata(&external).is_ok() {
                    builder = builder.add_source(
                        File::with_name(external.to_str().unwrap()).required(false),
                    );
                }
            }
        }

        // 3. load environment variables
        builder = builder.add_source(Environment::with_prefix("APP").separator("__"));

        // 构建并 Deserialize 到 Settings
        builder.build()?.try_deserialize()
    }
}
