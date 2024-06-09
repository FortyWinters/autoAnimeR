use crate::api::do_anime_task::handle_error;
use anyhow::Error;
use serde::{Deserialize, Serialize};
use serde_yml;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{Read, Seek, Write};
use std::path::Path;

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
    pub img_path: String,
    pub ui_url: String,
    pub qb_config: QbConfig,
}
async fn read_raw_config_file(path: &str) -> Result<String, Error> {
    let path = Path::new(path);
    let mut file = OpenOptions::new()
        .read(true)
        .open(path)
        .map_err(|e| handle_error(e, "Failed to open config file."))?;

    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .map_err(|e| handle_error(e, "Failed to read config file."))?;
    Ok(contents)
}

impl Config {
    #[allow(dead_code)]
    pub async fn load_config(path: &str) -> Result<Config, Error> {
        match read_raw_config_file(path).await {
            Ok(contents) => {
                let config = serde_yml::from_str(&contents)
                    .map_err(|e| handle_error(e, "Failed to parse config file."))?;
                log::info!("Successfully load config from {}, config detail: {:?}", path, config);
                Ok(config)
            }
            Err(e) => {
                log::warn!("Failed to reload config from: {}", path);
                Err(e)
            }
        }
    }
    
    #[allow(dead_code)]
    pub async fn reload_config(&mut self) -> Result<(), Error> {
        let path = "./config/config.yaml";
        if let Ok(contents) = read_raw_config_file(path).await {
            let new_config = serde_yml::from_str(&contents)
                .map_err(|e| handle_error(e, "Failed to parse config file."))?;
            let _ = std::mem::replace(self, new_config);
            log::info!("Successfully reload config from {}, new config detail: {:?}", path, self);
        } else {
            log::warn!("Failed to reload config from: {}", path);
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn modify_filed(
        &mut self,
        modify_list: &HashMap<String, String>,
    ) -> Result<(), Error> {
        for (filed, val) in modify_list.into_iter() {
            match filed.as_str() {
                "download_path" => {
                    self.download_path = val.to_string();
                    log::info!("update download_path: {}", val);
                }
                "img_path" => {
                    self.img_path = val.to_string();
                    log::info!("update img_path: {}", val);
                }
                "ui_url" => {
                    self.ui_url = val.to_string();
                    log::info!("update ui_url: {}", val);
                }
                "qb_url" => {
                    self.qb_config.qb_url = val.to_string();
                    log::info!("update qb_url: {}", val);
                }
                "username" => {
                    self.qb_config.username = val.to_string();
                    log::info!("update username: {}", val);
                }
                "password" => {
                    self.qb_config.password = val.to_string();
                    log::info!("update password: {}", val);
                }
                _ => {}
            }
        }

        let path = Path::new("./config/config.yaml");
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(|e| handle_error(e, "Failed to open config file."))?;

        file.seek(std::io::SeekFrom::Start(0))
            .map_err(|e| handle_error(e, "seek error"))?;
        file.set_len(0)
            .map_err(|e| handle_error(e, "set len error"))?;
        file.write_all(serde_yml::to_string(&self).unwrap().as_bytes())
            .map_err(|e| handle_error(e, "Failed to update video config file."))?;

        Ok(())
    }
}
#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    pub async fn test() {
        let mut config = Config::load_config("./config/config.yaml").await.unwrap();
        let mut modify_list = HashMap::new();
        modify_list.insert("download_path".to_string(), "downloads".to_string());
        config.modify_filed(&modify_list).await.unwrap();
        println!("{:?}", config);
    }
}
