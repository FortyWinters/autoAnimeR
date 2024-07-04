use crate::api::do_anime_task::handle_error;
use anyhow::Error;
use serde::{Deserialize, Serialize};
use serde_yml;
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
pub struct SubgroupFilter {
    pub preference: Vec<i32>,
    pub avoid: Vec<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimeConfig {
    pub subgroup_filter: SubgroupFilter,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub deploy_mode: String,
    pub download_path: String,
    pub img_path: String,
    pub ui_url: String,
    pub qb_config: QbConfig,
    pub anime_config: AnimeConfig,
}

trait Updatable {
    fn update_from(&mut self, other: &mut Self);
}

impl Updatable for QbConfig {
    fn update_from(&mut self, other: &mut Self) {
        if other.qb_url != "" {
            self.qb_url = std::mem::take(&mut other.qb_url);
        }
        if other.username != "" {
            self.username = std::mem::take(&mut other.username);
        }
        if other.password != "" {
            self.password = std::mem::take(&mut other.password);
        }
    }
}

impl Updatable for SubgroupFilter {
    fn update_from(&mut self, other: &mut Self) {
        if !other.preference.is_empty() {
            self.preference = std::mem::take(&mut other.preference);
        }
        if !other.avoid.is_empty() {
            self.avoid = std::mem::take(&mut other.avoid);
        }
    }
}

impl Updatable for AnimeConfig {
    fn update_from(&mut self, other: &mut Self) {
        self.subgroup_filter.update_from(&mut other.subgroup_filter);
    }
}

impl Updatable for Config {
    fn update_from(&mut self, other: &mut Self) {
        if other.deploy_mode != "" {
            self.deploy_mode = std::mem::take(&mut other.deploy_mode);
        }
        if other.download_path != "" {
            self.download_path = std::mem::take(&mut other.download_path);
        }
        if other.img_path != "" {
            self.img_path = std::mem::take(&mut other.img_path);
        }
        if other.ui_url != "" {
            self.ui_url = std::mem::take(&mut other.ui_url);
        }
        self.qb_config.update_from(&mut other.qb_config);
        self.anime_config.update_from(&mut other.anime_config);
    }
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
                log::info!(
                    "Successfully load config from {}, config detail: {:?}",
                    path,
                    config
                );
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
            log::info!(
                "Successfully reload config from {}, new config detail: {:?}",
                path,
                self
            );
        } else {
            log::warn!("Failed to reload config from: {}", path);
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn modify_filed(&mut self, new_config_val: &mut Config) -> Result<(), Error> {
        self.update_from(new_config_val);

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
        let mut new_config_val = Config {
            deploy_mode: "".to_string(),
            download_path: "".to_string(),
            img_path: "".to_string(),
            ui_url: "".to_string(),
            qb_config: QbConfig {
                qb_url: "".to_string(),
                username: "".to_string(),
                password: "".to_string(),
            },
            anime_config: AnimeConfig {
                subgroup_filter: SubgroupFilter {
                    preference: vec![123],
                    avoid: vec![456],
                },
            },
        };
        config.modify_filed(&mut new_config_val).await.unwrap();
        println!("{:?}", config);
    }
}
