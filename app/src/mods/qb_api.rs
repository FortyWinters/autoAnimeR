use crate::models::anime_seed::AnimeSeed;
use crate::error::error::AnimeError;
use chrono::DateTime;
use reqwest::multipart::{Form, Part};
use serde::{Deserialize, Serialize};
use serde_json;
use std::time::{Duration, UNIX_EPOCH};
#[derive(Debug, Clone)]
pub struct QbitTaskExecutor {
    pub qbt_client: reqwest::Client,
    pub cookie: String,
    pub host: String,
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
            .form(&[("username", username), ("password", password)])
            .send()
            .await
        {
            if resp.status().is_success() {
                match resp.cookies().next() {
                    Some(cookie) => Ok(Self {
                        qbt_client,
                        cookie: format!("{}={}", cookie.name(), cookie.value()),
                        host,
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

    pub async fn qb_api_version(&self) -> Result<(), AnimeError> {
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
        torrent_name: &String,
    ) -> Result<TorrentInfo, AnimeError> {
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
            let torrent_info = TorrentInfo::new(&json[0]).unwrap();
            Ok(torrent_info)
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
        anime_name: &String,
        anime_seed_info: &AnimeSeed,
    ) -> Result<(), AnimeError> {
        let add_endpoint = self.host.clone() + "api/v2/torrents/add";
        let file_name = anime_seed_info
            .seed_url
            .rsplit('/')
            .next()
            .unwrap_or(&anime_seed_info.seed_url)
            .to_string();

        let seed_path = format!("downloads/seed/{}/{}", anime_seed_info.mikan_id, file_name);
        let file_byte = std::fs::read(seed_path).unwrap();
        let form = Form::new()
            .part("torrent", Part::bytes(file_byte).file_name(file_name))
            .text("savepath", anime_name.clone());

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

    pub async fn qb_api_del_torrent(&self, torrent_name: &String) -> Result<(), AnimeError> {
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
        anime_name: &String,
        subgroup_name: &String,
        anime_seed_info: &AnimeSeed,
    ) -> Result<(), AnimeError> {
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

    pub async fn qb_api_resume_torrent(&self, torrent_name: &String) -> Result<(), AnimeError> {
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

    pub async fn qb_api_pause_torrent(&self, torrent_name: &String) -> Result<(), AnimeError> {
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
            let json: serde_json::Value =
                serde_json::from_str(&completed_torrent_response_text).unwrap();

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

    pub async fn qb_api_get_download_path(&self) -> Result<String, AnimeError> {
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
        Ok(download_path)
    }
}
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
        let item_size = item["size"].as_i64().ok_or("Field not found").unwrap();

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
    async fn test_qb_api_version() {
        let qb_task_executor =
            QbitTaskExecutor::new_with_login("admin".to_string(), "adminadmin".to_string())
                .await
                .unwrap();

        qb_task_executor.qb_api_version().await.unwrap();
    }

    #[tokio::test]
    async fn test_qb_api_torrent_info() {
        let torrent_name = "bdd2f547cdfd8a38011a5ea451d65379c9572305.torrent".to_string();

        let qb_task_executor =
            QbitTaskExecutor::new_with_login("admin".to_string(), "adminadmin".to_string())
                .await
                .unwrap();

        let r = qb_task_executor
            .qb_api_torrent_info(&torrent_name)
            .await
            .unwrap();
        println!("{}", r.state);
    }

    #[tokio::test]
    async fn test_qb_api_add_torrent() {
        let qb_task_executor =
            QbitTaskExecutor::new_with_login("admin".to_string(), "adminadmin".to_string())
                .await
                .unwrap();

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

    #[tokio::test]
    async fn test_qb_api() {
        let qb_task_executor =
            QbitTaskExecutor::new_with_login("admin".to_string(), "adminadmin".to_string())
                .await
                .unwrap();

        let r = qb_task_executor
            .qb_api_completed_torrent_list()
            .await
            .unwrap();

        let _r = qb_task_executor.qb_api_get_download_path().await.unwrap();

        println!("{:?}", r.len())
    }
}
