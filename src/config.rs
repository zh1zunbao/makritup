//! Global configuration management
//! This module handles the loading and parsing of configuration files
//! and environment variables for the application.
//! Usage:
//! ```rust
//! use markitup::config::SETTINGS;
//! // fn main() {
//! //     let cfg = &*SETTINGS.read().unwrap();
//! //     println!("{:?}", cfg.model_path);
//! // }

use config::{Config, ConfigError, Environment, File, FileFormat};
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::{env, fs, path::PathBuf, sync::RwLock};

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub model_path: PathBuf,
    pub image_path: PathBuf,
    pub output_path: Option<PathBuf>,
    pub is_ai_enpower: bool,
    pub doubao_api_key: Option<String>,
}

pub static SETTINGS: Lazy<RwLock<Settings>> = Lazy::new(|| {
    let settings = Settings::new().unwrap_or_else(|e| {
        eprintln!("Failed to load configuration: {}", e);
        std::process::exit(1);
    });
    
    // Debug output for all configuration settings
    if cfg!(debug_assertions) {
        println!("=== Configuration Settings ===");
        println!("model_path: {:?}", settings.model_path);
        println!("image_path: {:?}", settings.image_path);
        println!("output_path: {:?}", settings.output_path);
        println!("is_ai_enpower: {}", settings.is_ai_enpower);
        println!("doubao_api_key: {:?}", settings.doubao_api_key.as_ref());
        println!("==============================");
    }
    
    RwLock::new(settings)
});

// 提供一个便捷的访问函数，保持原有的使用方式
pub fn get_settings() -> Settings {
    SETTINGS.read().unwrap().clone()
}

// 添加更新配置的函数
pub fn update_settings_with_cli_args(
    image_path: Option<PathBuf>,
    output_path: Option<PathBuf>,
    ai_enable: Option<bool>,
) {
    let mut settings = SETTINGS.write().unwrap();

    if let Some(path) = image_path {
        settings.image_path = path;
    }

    if let Some(path) = output_path {
        settings.output_path = Some(path);
    }

    if let Some(enable) = ai_enable {
        settings.is_ai_enpower = enable;
    }
    
    // Debug output after CLI updates
    if cfg!(debug_assertions) {
        println!("=== Updated Configuration Settings ===");
        println!("model_path: {:?}", settings.model_path);
        println!("image_path: {:?}", settings.image_path);
        println!("output_path: {:?}", settings.output_path);
        println!("is_ai_enpower: {}", settings.is_ai_enpower);
        println!("doubao_api_key: {:?}", settings.doubao_api_key.as_ref());
        println!("=====================================");
    }
}

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
