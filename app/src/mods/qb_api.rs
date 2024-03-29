use crate::models::anime_seed::AnimeSeed;
use chrono::DateTime;
use reqwest::{
    multipart::{Form, Part},
    Error,
};
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
    pub async fn new_with_login(username: String, password: String) -> Result<Self, Error> {
        let qbt_client = reqwest::Client::new();
        // let host = "http://172.172.0.2:8081/".to_string();
        let host = "http://127.0.0.1:8081/".to_string();
        let login_endpoint = host.clone() + "api/v2/auth/login";

        let resp = qbt_client
            .post(login_endpoint)
            .header("Referer", &host)
            .form(&[("username", username), ("password", password)])
            .send()
            .await?;

        if resp.status().is_success() {
            match resp.cookies().next() {
                Some(cookie) => Ok(Self {
                    qbt_client,
                    cookie: format!("{}={}", cookie.name(), cookie.value()),
                    host,
                }),
                None => panic!("without cookies found"),
            }
        } else {
            panic!("status is not success");
        }
    }

    pub async fn qb_api_version(&self) -> Result<(), Error> {
        let webapiversion_endpoint = self.host.clone() + "api/v2/app/webapiVersion";
        let info_response = self
            .qbt_client
            .get(webapiversion_endpoint)
            .header("Cookie", &self.cookie)
            .send()
            .await?;

        log::info!("Response Status: {}", info_response.status());
        log::info!("Response Body: {}", info_response.text().await?);
        Ok(())
    }

    pub async fn qb_api_torrent_info(&self, torrent_name: &String) -> Result<TorrentInfo, Error> {
        let torrent_info_endpoint = self.host.clone() + "api/v2/torrents/info";
        let hashes = torrent_name.split('.').next().unwrap().to_owned();

        let torrent_info_response = self
            .qbt_client
            .post(torrent_info_endpoint)
            .header("Cookie", &self.cookie)
            .form(&[("hashes", &hashes)])
            .send()
            .await?;

        let torrent_info_response_text = torrent_info_response.text().await?;
        let json: serde_json::Value = serde_json::from_str(&torrent_info_response_text).unwrap();
        let torrent_info = TorrentInfo::new(&json[0])?;

        Ok(torrent_info)
    }

    pub async fn qb_api_add_torrent(
        &self,
        anime_name: &String,
        anime_seed_info: &AnimeSeed,
    ) -> Result<(), Error> {
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

        let add_response = self
            .qbt_client
            .post(add_endpoint)
            .header("Cookie", &self.cookie)
            .multipart(form)
            .send()
            .await?;

        log::info!("Response Status: {}", add_response.status());
        log::info!("Response Body: {}", add_response.text().await?);

        Ok(())
    }

    pub async fn qb_api_del_torrent(&self, torrent_name: &String) -> Result<(), Error> {
        let delete_endpoint = self.host.clone() + "api/v2/torrents/delete";
        let hashes = torrent_name.split('.').next().unwrap().to_owned();

        let delete_response = self
            .qbt_client
            .post(delete_endpoint)
            .header("Cookie", &self.cookie)
            .form(&[("hashes", hashes), ("deleteFiles", String::from("false"))])
            .send()
            .await?;

        log::info!("Response Status: {}", delete_response.status());
        log::info!("Response Body: {}", delete_response.text().await?);

        Ok(())
    }

    pub async fn qb_api_torrent_rename_file(
        &self,
        anime_name: &String,
        subgroup_name: &String,
        anime_seed_info: &AnimeSeed,
    ) -> Result<(), Error> {
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

        let torrent_info_response = self
            .qbt_client
            .post(rename_file_endpoint)
            .header("Cookie", &self.cookie)
            .form(&[
                ("hash", hashes),
                ("oldPath", format!("{}", file_name.to_string())),
                (
                    "newPath",
                    format!(
                        "{}{}{}{}{}{}{}",
                        anime_name,
                        " - ",
                        anime_seed_info.episode,
                        " - ",
                        subgroup_name,
                        ".",
                        extension
                    ),
                ),
            ])
            .send()
            .await?;
        log::info!("Response Status: {}", torrent_info_response.status());
        log::info!("Response Body: {}", torrent_info_response.text().await?);
        Ok(())
    }

    pub async fn qb_api_resume_torrent(&self, torrent_name: &String) -> Result<(), Error> {
        let resume_endpoint = self.host.clone() + "api/v2/torrents/resume";
        let hashes = torrent_name.split('.').next().unwrap().to_owned();

        let resume_response = self
            .qbt_client
            .post(resume_endpoint)
            .header("Cookie", &self.cookie)
            .form(&[("hashes", hashes)])
            .send()
            .await?;

        log::info!("Response Status: {}", resume_response.status());
        log::info!("Response Body: {}", resume_response.text().await?);
        Ok(())
    }

    pub async fn qb_api_pause_torrent(&self, torrent_name: &String) -> Result<(), Error> {
        let pause_endpoint = self.host.clone() + "api/v2/torrents/pause";
        let hashes = torrent_name.split('.').next().unwrap().to_owned();

        let pause_response = self
            .qbt_client
            .post(pause_endpoint)
            .header("Cookie", &self.cookie)
            .form(&[("hashes", hashes)])
            .send()
            .await?;

        log::info!("Response Status: {}", pause_response.status());
        log::info!("Response Body: {}", pause_response.text().await?);

        Ok(())
    }

    pub async fn qb_api_completed_torrent_list(&self) -> Result<Vec<String>, Error> {
        let torrent_info_endpoint = self.host.clone() + "api/v2/torrents/info";

        let completed_torrent_response = self
            .qbt_client
            .post(torrent_info_endpoint)
            .header("Cookie", &self.cookie)
            .form(&[("filter", "completed")])
            .send()
            .await?;

        let completed_torrent_response_text = completed_torrent_response.text().await?;
        let json: serde_json::Value =
            serde_json::from_str(&completed_torrent_response_text).unwrap();
        let mut torrent_hash_list: Vec<String> = Vec::new();

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

        Ok(torrent_hash_list)
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
    pub fn new(item: &serde_json::Value) -> Result<Self, Error> {
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
        println!("{:?}", r.len())
    }
}
