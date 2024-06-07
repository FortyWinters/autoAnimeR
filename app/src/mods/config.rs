use crate::api::do_anime_task::handle_error;
use anyhow::Error;
use serde_yml;
use std::fs::OpenOptions;
use std::io::Read;
use std::path::Path;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QbConfig {
    pub qb_url: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub deploy_mode: String,
    pub download_path: String,
    pub ui_url: String,
    pub qb_config: QbConfig,
}

impl Config {
    #[allow(dead_code)]
    pub fn load_config(path: &str) -> Result<Config, Error> {
        let path = Path::new(path);

        let mut file = OpenOptions::new()
            .read(true)
            .open(path)
            .map_err(|e| handle_error(e, "Failed to open config file."))?;

        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|e| handle_error(e, "Failed to read config file."))?;

        let config: Config = serde_yml::from_str(&contents)
            .map_err(|e| handle_error(e, "Failed to parse config file."))?;

        Ok(config)
    }
    // TODO: add reload func
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    pub async fn test() {
        let config = Config::load_config("./config/config.yaml").unwrap();
        println!("{:?}", config);
    }
}
