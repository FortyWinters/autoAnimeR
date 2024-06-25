use crate::api::do_anime_task::handle_error;
use anyhow::Error;
use serde::{Deserialize, Serialize};
use serde_yml;
use std::fs::OpenOptions;
use std::io::{Read, Seek, Write};
use std::path::Path;

macro_rules! update_fields {
    ($self:ident, $new_config_val:ident, { $($field:ident),* }) => {
        $(
            $self.$field = $new_config_val.$field.to_string();
        )*
    };
    ($self:ident, $new_config_val:ident, $config:ident, { $($field:ident),* }) => {
        $(
            $self.$config.$field = $new_config_val.$config.$field.to_string();
        )*
    };
}

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
        new_config_val: &Config,
    ) -> Result<(), Error> {
        update_fields!(self, new_config_val, { download_path, img_path, ui_url });
        update_fields!(self, new_config_val, qb_config, { qb_url, username, password });

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
        let new_config_val = Config {
            deploy_mode: "".to_string(),
            download_path: "".to_string(),
            img_path: "".to_string(),
            ui_url: "".to_string(),
            qb_config: QbConfig {
                qb_url: "".to_string(),
                username: "".to_string(),
                password: "".to_string(),
            },
        };
        config.modify_filed(&new_config_val).await.unwrap();
        println!("{:?}", config);
    }
}
