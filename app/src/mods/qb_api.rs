use crate::error::error::AnimeError;
use crate::models::anime_seed::AnimeSeed;
use crate::mods::config::Config;
use chrono::DateTime;
use reqwest::multipart::{Form, Part};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashSet;
use std::time::{Duration, UNIX_EPOCH};

pub fn handle_error<E: std::fmt::Debug>(e: E, _message: &str) -> AnimeError {
    AnimeError::new(format!("{:?}", e))
}

#[derive(Debug, Clone)]
pub struct QbitTaskExecutor {
    pub is_login: bool,
    qbt_client: reqwest::Client,
    cookie: String,
    host: String,
    download_path: String,
    deploy_mode: String,
}

#[allow(dead_code)]
impl QbitTaskExecutor {
    pub async fn new_with_login(username: String, password: String) -> Result<Self, AnimeError> {
        let qbt_client = reqwest::Client::new();
        let host = "http://127.0.0.1:8081/".to_string();
        let login_endpoint = host.clone() + "api/v2/auth/login";

        if let Ok(resp) = qbt_client
            .post(login_endpoint)
            .header("Referer", &host)
            .form(&[("username", &username), ("password", &password)])
            .send()
            .await
        {
            if resp.status().is_success() {
                match resp.cookies().next() {
                    Some(cookie) => Ok(Self {
                        qbt_client,
                        is_login: true,
                        cookie: format!("{}={}", cookie.name(), cookie.value()),
                        host,
                        download_path: "downloads".to_string(),
                        deploy_mode: "local".to_string(),
                    }),
                    None => panic!("[QB API] Login error, without cookies found"),
                }
            } else {
                panic!("[QB API] Login error, response status is {}", resp.status());
            }
        } else {
            panic!("[QB API] Login error, qbittorrent client error");
        }
    }

    pub async fn new_with_config(config: &Config) -> Result<Self, AnimeError> {
        let qbt_client = reqwest::Client::new();
        let host = config.qb_config.qb_url.to_string();
        let login_endpoint = host.clone() + "api/v2/auth/login";

        if let Ok(resp) = qbt_client
            .post(login_endpoint)
            .header("Referer", &host)
            .form(&[
                ("username", &config.qb_config.username),
                ("password", &config.qb_config.password),
            ])
            .send()
            .await
        {
            if resp.status().is_success() {
                match resp.cookies().next() {
                    Some(cookie) => {
                        log::info!("[QB API] Successfully Connect to qbittorrent web api");
                        return Ok(Self {
                            qbt_client,
                            is_login: true,
                            cookie: format!("{}={}", cookie.name(), cookie.value()),
                            host,
                            download_path: config.download_path.clone(),
                            deploy_mode: config.deploy_mode.clone(),
                        });
                    }
                    None => log::error!("[QB API] Login error, without cookies found"),
                }
            } else {
                log::error!("[QB API] Login error, response status is {}", resp.status());
            }
        } else {
            log::error!("[QB API] Login error, qbittorrent client error");
        }

        Ok(Self {
            qbt_client,
            is_login: false,
            cookie: "".to_string(),
            host,
            download_path: config.download_path.clone(),
            deploy_mode: config.deploy_mode.clone(),
        })
    }

    pub async fn relogin(&mut self, config: &Config) -> Result<(), AnimeError> {
        self.deploy_mode = config.deploy_mode.clone();
        self.host = config.qb_config.qb_url.clone();
        self.download_path = config.download_path.clone();

        let login_endpoint = self.host.clone() + "api/v2/auth/login";

        if let Ok(resp) = self
            .qbt_client
            .post(login_endpoint)
            .header("Referer", &self.host)
            .form(&[
                ("username", config.qb_config.username.clone()),
                ("password", config.qb_config.password.clone()),
            ])
            .send()
            .await
        {
            if resp.status().is_success() {
                match resp.cookies().next() {
                    Some(cookie) => {
                        log::info!("[QB API] Successfully Connect to qbittorrent web api");
                        self.cookie = format!("{}={}", cookie.name(), cookie.value());
                        self.is_login = true;
                        return Ok(());
                    }
                    None => log::error!("[QB API] Login error, without cookies found"),
                }
            } else {
                log::error!("[QB API] Login error, response status is {}", resp.status());
            }
        } else {
            log::error!("[QB API] Login error, qbittorrent client error");
        }

        Err(AnimeError::new("[QB API]relogin failed".to_string()))
    }

    pub async fn qb_api_version(&self) -> Result<(), AnimeError> {
        if !self.is_login {
            return Err(AnimeError::new(
                "[QB API] qbittorrent client not started".to_string(),
            ));
        }

        let webapiversion_endpoint = self.host.clone() + "api/v2/app/webapiVersion";
        if let Ok(info_response) = self
            .qbt_client
            .get(webapiversion_endpoint.clone())
            .header("Cookie", &self.cookie)
            .send()
            .await
        {
            log::info!(
                "[QB API] qb_api_version: {}",
                info_response.text().await.unwrap()
            );
        } else {
            log::info!(
                "[QB API] Unable to access qb web api: {}",
                webapiversion_endpoint
            );
        }
        Ok(())
    }

    pub async fn qb_api_torrent_info(
        &self,
        torrent_name: &str,
    ) -> Result<TorrentInfo, AnimeError> {
        if !self.is_login {
            return Err(AnimeError::new(
                "[QB API] qbittorrent client not started".to_string(),
            ));
        }

        let torrent_info_endpoint = self.host.clone() + "api/v2/torrents/info";
        let hashes = torrent_name.split('.').next().unwrap().to_owned();

        if let Ok(torrent_info_response) = self
            .qbt_client
            .post(torrent_info_endpoint.clone())
            .header("Cookie", &self.cookie)
            .form(&[("hashes", &hashes)])
            .send()
            .await
        {
            let torrent_info_response_text = torrent_info_response.text().await.unwrap();
            let json: serde_json::Value =
                serde_json::from_str(&torrent_info_response_text).unwrap();
            match TorrentInfo::new(&json[0]) 
            {
                Ok(torrent_info) => Ok(torrent_info),
                Err(e) => Err(e)
            }
        } else {
            log::info!(
                "[QB API] Unable to access qb web api: {}",
                torrent_info_endpoint
            );
            Err(AnimeError::new(format!(
                "[QB API] Unable to access qb web api: {}",
                torrent_info_endpoint,
            )))
        }
    }

    pub async fn qb_api_add_torrent(
        &self,
        anime_name: &str,
        anime_seed_info: &AnimeSeed,
    ) -> Result<(), AnimeError> {
        if !self.is_login {
            return Err(AnimeError::new(
                "[QB API] qbittorrent client not started".to_string(),
            ));
        }

        let add_endpoint = self.host.clone() + "api/v2/torrents/add";
        let file_name = anime_seed_info
            .seed_url
            .rsplit('/')
            .next()
            .unwrap_or(&anime_seed_info.seed_url)
            .to_string();

        println!("{}", anime_seed_info.seed_name);

        let seed_path = format!("downloads/seed/{}/{}", anime_seed_info.mikan_id, file_name);
        let file_byte = std::fs::read(seed_path).unwrap();
        let save_path = anime_name.to_string() + "(" + &anime_seed_info.mikan_id.to_string() + ")";
        let form = Form::new()
            .part("torrent", Part::bytes(file_byte).file_name(file_name))
            .text("savepath", save_path);

        if let Ok(_) = self
            .qbt_client
            .post(add_endpoint.clone())
            .header("Cookie", &self.cookie)
            .multipart(form)
            .send()
            .await
        {
            log::info!("[QB API] Successfully added seeds: {:?}", anime_seed_info);
        } else {
            log::info!("[QB API] Unable to access qb web api: {}", add_endpoint);
        }
        Ok(())
    }

    pub async fn qb_api_del_torrent(&self, torrent_name: &str) -> Result<(), AnimeError> {
        if !self.is_login {
            return Err(AnimeError::new(
                "[QB API] qbittorrent client not started".to_string(),
            ));
        }

        let delete_endpoint = self.host.clone() + "api/v2/torrents/delete";
        let hashes = torrent_name.split('.').next().unwrap().to_owned();

        if let Ok(_) = self
            .qbt_client
            .post(delete_endpoint.clone())
            .header("Cookie", &self.cookie)
            .form(&[("hashes", hashes), ("deleteFiles", String::from("false"))])
            .send()
            .await
        {
            log::info!("[QB API] Successfully delete seeds: {}", torrent_name);
        } else {
            log::info!("[QB API] Unable to access qb web api: {}", delete_endpoint);
        }
        Ok(())
    }

    pub async fn qb_api_torrent_rename_file(
        &self,
        anime_name: &str,
        subgroup_name: &str,
        anime_seed_info: &AnimeSeed,
    ) -> Result<(), AnimeError> {
        if !self.is_login {
            return Err(AnimeError::new(
                "[QB API] qbittorrent client not started".to_string(),
            ));
        }

        let rename_file_endpoint = self.host.clone() + "api/v2/torrents/renameFile";
        let torrent_name = anime_seed_info
            .seed_url
            .rsplit('/')
            .next()
            .unwrap_or(&anime_seed_info.seed_url)
            .to_string();
        let hashes = torrent_name.split('.').next().unwrap().to_owned();
        let file_name = self.qb_api_torrent_info(&torrent_name).await.unwrap().name;
        let extension = match file_name.rsplit('.').next() {
            Some(ext) => ext,
            None => "mp4",
        };

        let new_name = format!(
            "{}{}{}{}{}{}{}",
            anime_name, " - ", anime_seed_info.episode, " - ", subgroup_name, ".", extension
        );

        if let Ok(torrent_info_response) = self
            .qbt_client
            .post(rename_file_endpoint.clone())
            .header("Cookie", &self.cookie)
            .form(&[
                ("hash", hashes),
                ("oldPath", file_name.to_string()),
                ("newPath", new_name.clone()),
            ])
            .send()
            .await
        {
            if torrent_info_response.status().is_success() {
                log::info!(
                    "[QB API] Successfully rename seeds: {} with new name: {}",
                    file_name,
                    new_name
                );
            } else {
                log::info!(
                    "[QB API] Unalble to rename seed: {} with new name: {}, {}",
                    file_name,
                    new_name,
                    torrent_info_response.text().await.unwrap()
                )
            }
        } else {
            log::info!(
                "[QB API] Unable to access qb web api: {}",
                rename_file_endpoint
            );
        }
        Ok(())
    }

    pub async fn qb_api_resume_torrent(&self, torrent_name: &str) -> Result<(), AnimeError> {
        if !self.is_login {
            return Err(AnimeError::new(
                "[QB API] qbittorrent client not started".to_string(),
            ));
        }

        let resume_endpoint = self.host.clone() + "api/v2/torrents/resume";
        let hashes = torrent_name.split('.').next().unwrap().to_owned();

        if let Ok(_) = self
            .qbt_client
            .post(resume_endpoint.clone())
            .header("Cookie", &self.cookie)
            .form(&[("hashes", hashes)])
            .send()
            .await
        {
            log::info!("[QB API] Successfully resume seed: {}", torrent_name);
        } else {
            log::info!("[QB API] Unable to access qb web api: {}", resume_endpoint);
        }
        Ok(())
    }

    pub async fn qb_api_pause_torrent(&self, torrent_name: &str) -> Result<(), AnimeError> {
        if !self.is_login {
            return Err(AnimeError::new(
                "[QB API] qbittorrent client not started".to_string(),
            ));
        }

        let pause_endpoint = self.host.clone() + "api/v2/torrents/pause";
        let hashes = torrent_name.split('.').next().unwrap().to_owned();

        if let Ok(_) = self
            .qbt_client
            .post(pause_endpoint.clone())
            .header("Cookie", &self.cookie)
            .form(&[("hashes", hashes)])
            .send()
            .await
        {
            log::info!("[QB API] Successfully pause seed: {}", torrent_name);
        } else {
            log::info!("[QB API] Unable to access qb web api: {}", pause_endpoint);
        }
        Ok(())
    }

    pub async fn qb_api_completed_torrent_list(&self) -> Result<Vec<String>, AnimeError> {
        if !self.is_login {
            return Err(AnimeError::new(
                "[QB API] qbittorrent client not started".to_string(),
            ));
        }

        let torrent_info_endpoint = self.host.clone() + "api/v2/torrents/info";
        let mut torrent_hash_list: Vec<String> = Vec::new();

        if let Ok(completed_torrent_response) = self
            .qbt_client
            .post(torrent_info_endpoint.clone())
            .header("Cookie", &self.cookie)
            .form(&[("filter", "completed")])
            .send()
            .await
        {
            let completed_torrent_response_text = completed_torrent_response.text().await.unwrap();
            let json: serde_json::Value = serde_json::from_str(&completed_torrent_response_text)
                .map_err(|e| {
                    handle_error(
                        e,
                        format!("Failed to serialize {:?}", completed_torrent_response_text)
                            .as_str(),
                    )
                })?;

            if let serde_json::Value::Array(torrents) = json {
                for torrent in torrents {
                    torrent_hash_list.push(
                        torrent["hash"]
                            .as_str()
                            .ok_or("Field not found")
                            .unwrap()
                            .to_string()
                            + ".torrent",
                    );
                }
            }
        } else {
            log::info!(
                "[QB API] Unable to access qb web api: {}",
                torrent_info_endpoint
            );
        }
        Ok(torrent_hash_list)
    }

    pub async fn qb_api_completed_torrent_set(&self) -> Result<HashSet<String>, AnimeError> {
        if !self.is_login {
            return Err(AnimeError::new(
                "[QB API] qbittorrent client not started".to_string(),
            ));
        }

        let torrent_info_endpoint = self.host.clone() + "api/v2/torrents/info";
        let mut torrent_hash_set: HashSet<String> = HashSet::new();

        if let Ok(completed_torrent_response) = self
            .qbt_client
            .post(torrent_info_endpoint.clone())
            .header("Cookie", &self.cookie)
            .form(&[("filter", "completed")])
            .send()
            .await
        {
            let completed_torrent_response_text = completed_torrent_response.text().await.unwrap();
            let json: serde_json::Value =
                serde_json::from_str(&completed_torrent_response_text).unwrap();

            if let serde_json::Value::Array(torrents) = json {
                for torrent in torrents {
                    torrent_hash_set.insert(
                        torrent["hash"]
                            .as_str()
                            .ok_or("Field not found")
                            .unwrap()
                            .to_string()
                            + ".torrent",
                    );
                }
            }
        } else {
            log::info!(
                "[QB API] Unable to access qb web api: {}",
                torrent_info_endpoint
            );
        }
        Ok(torrent_hash_set)
    }

    pub async fn qb_api_get_download_path(&self) -> Result<String, AnimeError> {
        if !self.is_login {
            return Ok(self.download_path.clone());
        }

        let app_info_endpoint = self.host.clone() + "api/v2/app/preferences";
        let mut download_path = String::from("");
        if let Ok(app_info_response) = self
            .qbt_client
            .get(app_info_endpoint.clone())
            .header("Cookie", &self.cookie)
            .send()
            .await
        {
            let app_info_response_text = app_info_response.text().await.unwrap();
            let json: serde_json::Value = serde_json::from_str(&app_info_response_text).unwrap();
            download_path = json["save_path"].to_string().replace("\"", "");
            log::info!("download path: {:?}", download_path);
        } else {
            log::info!(
                "[QB API] Unable to access qb web api: {}",
                app_info_endpoint
            );
        }

        match self.deploy_mode.as_str() {
            "docker" => Ok(self.download_path.clone()),
            _ => Ok(download_path),
        }
    }
}

unsafe impl Send for QbitTaskExecutor {}

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct TorrentInfo {
    pub name: String,
    pub size: String,
    pub done: String,           // progress
    pub peers: String,          // num_leechs
    pub seeds: String,          // num_seeds
    pub download_speed: String, // dlspeed
    pub eta: String,
    pub hash: String,
    pub state: String,
}

impl TorrentInfo {
    pub fn new(item: &serde_json::Value) -> Result<Self, AnimeError> {
        let item_size = item["size"]
            .as_i64()
            .ok_or("Field not found")
            .map_err(|e| handle_error(e, "No item named 'size'"))?;

        const GB: i64 = 1024 * 1024 * 1024;
        const MB: i64 = 1024 * 1024;
        let size = if item_size >= GB {
            format!("{:.2} GB", item_size as f64 / GB as f64)
        } else {
            format!("{:.2} MB", item_size as f64 / MB as f64)
        };

        let item_dlspeed = item["dlspeed"].as_i64().ok_or("Field not found").unwrap();

        const KILOBIT: i64 = 1000;
        const MEGABIT: i64 = 1000000;
        let download_speed = if item_dlspeed > MEGABIT {
            format!("{:.2} Mbps", item_dlspeed as f64 / MEGABIT as f64)
        } else {
            format!("{:.2} Kbps", item_dlspeed as f64 / KILOBIT as f64)
        };

        let item_eta = item["eta"].as_i64().ok_or("Field not found").unwrap();

        let d = UNIX_EPOCH + Duration::from_secs(item_eta as u64);
        let datetime = DateTime::<chrono::Utc>::from(d);
        let eta: String = datetime.format("%H:%M:%S").to_string();

        Ok(TorrentInfo {
            name: item["name"]
                .as_str()
                .ok_or("Field not found")
                .unwrap()
                .to_owned(),
            size,
            done: format!(
                "{:.2} %",
                (item["progress"].as_f64().ok_or("Field not found").unwrap() * 100.0)
            ),
            peers: item["num_leechs"]
                .as_i64()
                .ok_or("Field not found")
                .unwrap()
                .to_string(),
            seeds: item["num_seeds"]
                .as_i64()
                .ok_or("Field not found")
                .unwrap()
                .to_string(),
            download_speed,
            eta,
            hash: item["hash"]
                .as_str()
                .ok_or("Field not found")
                .unwrap()
                .to_owned(),
            state: item["state"]
                .as_str()
                .ok_or("Field not found")
                .unwrap()
                .to_owned(),
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_qb_api_add_torrent() {
        let config = Config::load_config("./config/config.json").await.unwrap();
        let qb_task_executor = QbitTaskExecutor::new_with_config(&config).await.unwrap();

        let anime_name = "test".to_string();
        let anime_seed_info = AnimeSeed {
            id: Some(100),
            mikan_id: 3143,
            subgroup_id: 382,
            episode: 3,
            seed_name:
                "【喵萌奶茶屋】★10月新番★[米基与达利 / Migi to Dali][03][1080p][简日双语][招募翻译]"
                    .to_string(),
            seed_url: "/Download/20231021/bdd2f547cdfd8a38011a5ea451d65379c9572305.torrent"
                .to_string(),
            seed_status: 0,
            seed_size: "349.4MB".to_string(),
        };

        qb_task_executor
            .qb_api_add_torrent(&anime_name, &anime_seed_info)
            .await
            .unwrap();
    }
}
