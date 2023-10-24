use reqwest::{ multipart::{ Part, Form }, Error };
use serde_json;
use chrono::DateTime;
use std::{
    collections::HashMap,
    time::{ Duration, UNIX_EPOCH }
};
use serde::{ Deserialize, Serialize };

#[derive(Debug)]
pub struct QbitTaskExecutor {
    pub qbt_client: reqwest::Client,
    pub cookie: String,
}

#[allow(dead_code)]
impl QbitTaskExecutor {
    pub async fn new_with_login(username: String, password: String) -> Result<Self, Error>
    {
        let qbt_client = reqwest::Client::new();
        let login_endpoint = "http://127.0.0.1:8081/api/v2/auth/login";

        let resp = qbt_client
            .post(login_endpoint)
            .header("Referer", "http://127.0.0.1:8081")
            .form(&[
                ("username", username),
                ("password", password),
            ])
            .send()
            .await?;

        if resp.status().is_success() {
            match resp.cookies().next(){
                Some(cookie) => {
                    Ok(Self {
                        qbt_client,
                        cookie: format!("{}={}", cookie.name(), cookie.value()), 
                    })
                }
                None => panic!("without cookies found"),
            }
        }
        else{
            panic!("status is not success");
        }
    }

    pub async fn qb_api_version(&self) -> Result<(), Error> {
        let webapiversion_endpoint = "http://127.0.0.1:8081/api/v2/app/webapiVersion";
        let info_response = self.qbt_client
            .get(webapiversion_endpoint)
            .header("Cookie", &self.cookie)
            .send()
            .await?;

        println!("Response Status: {}", info_response.status());
        println!("Response Body:\n{}", info_response.text().await?);
        Ok(())
    }

    pub async fn qb_api_torrent_info(&self, torrent_name: String) -> Result<TorrentInfo, Error> {
        let torrent_info_endpoint = "http://127.0.0.1:8081/api/v2/torrents/info";
        let hashes = torrent_name
            .split('.')
            .next()
            .unwrap()
            .to_owned();

        let torrent_info_response = self.qbt_client
            .post(torrent_info_endpoint)
            .header("Cookie", &self.cookie)
            .query(&[("hashes", &hashes)])
            .send()
            .await?;

        let torrent_info_response_text = torrent_info_response.text().await?;
        let json: serde_json::Value = serde_json::from_str(&torrent_info_response_text).unwrap();
        let torrent_info = TorrentInfo::new(&json[0])?; 

        Ok(torrent_info)
    }

    pub async fn qb_api_add_torrent<>(&self, anime_name: &String, episode_info: &EpisodeInfo) -> Result<(), Error> {
        let add_endpoint = "http://127.0.0.1:8081/api/v2/torrents/add";
        let file_name = episode_info.seed_url
            .rsplit('/')
            .next()
            .unwrap()
            .to_string();
        let seed_path = format!("downloads/seed/{}/{}", episode_info.mikan_id, file_name);
        let file_byte = std::fs::read(seed_path).unwrap();
        let form = Form::new()
            .part("torrent", Part::bytes(file_byte).file_name(file_name))
            .text("savepath", anime_name.clone());

        let add_response = self.qbt_client
            .post(add_endpoint)
            .header("Cookie", &self.cookie)
            .multipart(form)
            .send()
            .await?;

        println!("Response Status: {}", add_response.status());
        println!("Response Body:\n{}", add_response.text().await?);
        
        Ok(())
    }
    
    pub async fn qb_api_del_torrent(&self, torrent_name: String) -> Result<(), Error> {
        let delete_endpoint = "http://127.0.0.1:8081/api/v2/torrents/delete";
        let hashes = torrent_name
            .split('.')
            .next()
            .unwrap()
            .to_owned();
        
        let delete_response = self.qbt_client
            .post(delete_endpoint)
            .header("Cookie", &self.cookie)
            .form(&[
                ("hashes", hashes),
                ("deleteFiles", String::from("false")),
            ])
            .send()
            .await?;

        println!("Response Status: {}", delete_response.status());
        println!("Response Body:\n{}", delete_response.text().await?);   

        Ok(())
    }

    pub async fn qb_api_torrent_rename_file(&self, anime_name: &String, episode_info: &EpisodeInfo) -> Result<(), Error> {
        let rename_file_endpoint = "http://127.0.0.1:8081/api/v2/torrents/renameFile";
        let hashes = episode_info.seed_url
            .rsplit('/')
            .next()
            .unwrap_or("")
            .trim_end_matches(".torrent")
            .to_string();
        let torrent_name = hashes.clone() + ".torrent";
        let file_name = self.qb_api_torrent_info(torrent_name)
            .await
            .unwrap()
            .name;
        let extension = match file_name.rsplit('.').next() { 
            Some(ext) => ext,
            None => ".mp4",
        };

        let torrent_info_response = self.qbt_client
            .post(rename_file_endpoint)
            .header("Cookie", &self.cookie)
            .form(&[
                ("hash", hashes),
                ("oldPath", format!("{}", file_name.to_string())),
                ("newPath", format!("{}{}{}{}{}{}{}",
                        anime_name, 
                        " - ", 
                        episode_info.episode, 
                        " - ",
                        episode_info.subgroup_name, 
                        ".",
                        extension)
                )])
            .send()
            .await?;

        println!("Response Status: {}", torrent_info_response.status());
        println!("Response Body:\n{}", torrent_info_response.text().await?);
        Ok(())
    }


    pub async fn qb_api_resume_torrent(&self, torrent_name: String) -> Result<(), Error> {
        let resume_endpoint = "http://127.0.0.1:8081/api/v2/torrents/resume";
        let hashes = torrent_name
            .split('.')
            .next()
            .unwrap()
            .to_owned();
        
        let resume_response = self.qbt_client
            .post(resume_endpoint)
            .header("Cookie", &self.cookie)
            .form(&[("hashes", hashes),])
            .send()
            .await?;

        println!("Response Status: {}", resume_response.status());
        println!("Response Body:\n{}", resume_response.text().await?);

        Ok(())
    }

    pub async fn qb_api_pause_torrent(&self, torrent_name: String) -> Result<(), Error> {
        let pause_endpoint = "http://127.0.0.1:8081/api/v2/torrents/pause";
        let hashes = torrent_name
            .split('.')
            .next()
            .unwrap()
            .to_owned();

        let pause_response = self.qbt_client
            .post(pause_endpoint)
            .header("Cookie", &self.cookie)
            .form(&[("hashes", hashes),])
            .send()
            .await?;

        println!("Response Status: {}", pause_response.status());
        println!("Response Body:\n{}", pause_response.text().await?);

        Ok(())
    }

    pub async fn qb_api_completed_torrent_list(&self) -> Result<Vec<String>, Error> {
        let torrent_info_endpoint = "http://127.0.0.1:8081/api/v2/torrents/info";

        let completed_torrent_response = self.qbt_client
            .post(torrent_info_endpoint)
            .header("Cookie", &self.cookie)
            .query(&[("filter", "completed")])
            .send()
            .await?;

        let completed_torrent_response_text = completed_torrent_response.text().await?;
        let json: serde_json::Value = serde_json::from_str(&completed_torrent_response_text).unwrap();
        let mut torrent_list: Vec<String> = Vec::new();

        if let serde_json::Value::Array(torrents) = json {
            for torrent in torrents {
                torrent_list.push(torrent["hash"]
                                .as_str()
                                .ok_or("Field not found")
                                .unwrap()
                                .to_string() + ".torrent"
                            );
            }
        } 

        Ok(torrent_list)
    }

}
#[derive(Default, Debug, Deserialize, Serialize)]
pub struct TorrentInfo {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "size")]
    pub size: String,
    #[serde(rename = "done")]
    pub done: String, // progress
    #[serde(rename = "peers")]
    pub peers: String, // num_leechs
    #[serde(rename = "seeds")]
    pub seeds: String, // num_seeds
    #[serde(rename = "download_speed")]
    pub download_speed: String, // dlspeed
    #[serde(rename = "eta")]
    pub eta: String,
    #[serde(rename = "hash")]
    pub hash: String,
}

impl TorrentInfo {
    pub fn new(item:& serde_json::Value) -> Result<Self, Error> {
        let item_size = item["size"]
                                .as_i64()
                                .ok_or("Field not found")
                                .unwrap();
        
        const GB: i64 = 1024 * 1024 * 1024;
        const MB: i64 = 1024 * 1024;
        let size = if item_size >= GB {
            format!("{:.2} GB", item_size as f64 / GB as f64)
        } else {
            format!("{:.2} MB", item_size as f64 / MB as f64)
        };

        let item_dlspeed = item["dlspeed"]
                                .as_i64()
                                .ok_or("Field not found")
                                .unwrap();
        
        const KILOBIT: i64 = 1000;
        const MEGABIT: i64 = 1000000;
        let download_speed = if item_dlspeed > MEGABIT {
            format!("{:.2} Mbps", item_dlspeed as f64 / MEGABIT as f64)
        } else {
            format!("{:.2} Kbps", item_dlspeed as f64 / KILOBIT as f64)
        };

        let item_eta = item["eta"]
                            .as_i64()
                            .ok_or("Field not found")
                            .unwrap();

        let d = UNIX_EPOCH + Duration::from_secs(item_eta as u64);
        let datetime = DateTime::<chrono::Utc>::from(d);
        let eta: String = datetime
                            .format("%Y-%m-%d %H:%M:%S.%f")
                            .to_string();
        
        Ok( TorrentInfo {
                name: item["name"]
                    .as_str()
                    .ok_or("Field not found")
                    .unwrap()
                    .to_owned(),
                size,
                done: (item["progress"]
                    .as_f64()
                    .ok_or("Field not found")
                    .unwrap() * 100.0)
                    .to_string() + "%",
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
                    .to_owned()
                })
    }
}

#[derive(Default, Clone)]
pub struct MikanTorrentsInfos {
    pub info: HashMap<i32, AnimeInfo> //<mikan_id, anime_info>
  }
  
#[derive(Clone)]
pub struct AnimeInfo {
    pub info: HashMap<i32, EpisodeInfo> //<episode, episode_info>
}

#[derive(Clone)]
pub struct EpisodeInfo {
    pub mikan_id: i32,
    pub episode: i32,
    pub subgroup_name: String,
    pub seed_name: String,
    pub seed_url: String,
}

#[derive(Clone)]
pub struct AnimeSeedInfo {
    pub mikan_id: i32,
    pub subgroup_name: String,
    pub episode: i32,
    pub seed_name: String,
    pub seed_url: String,
    pub seed_status: i32,
    pub seed_size: String
}

#[allow(dead_code)]
impl MikanTorrentsInfos {
    pub async fn new(anime_seed: AnimeSeedInfo, ) -> Self {
        
        let episode_info = EpisodeInfo {
            mikan_id:anime_seed.mikan_id,
            episode: anime_seed.episode,
            subgroup_name: anime_seed.subgroup_name,
            seed_name: anime_seed.seed_name,
            seed_url: anime_seed.seed_url,
        };
        
        let mut anime_info = AnimeInfo { info: HashMap::new() };
        anime_info.info.insert(anime_seed.episode, episode_info);
        
        let mut mikan_torrents_infos = MikanTorrentsInfos { info: HashMap::new() };
        mikan_torrents_infos.info.insert(anime_seed.mikan_id, anime_info);
        mikan_torrents_infos
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_qb_api_version() {
        let qb_task_executor = QbitTaskExecutor::new_with_login(
            "admin".to_string(), 
            "adminadmin".to_string())
            .await
            .unwrap();
        
        qb_task_executor.qb_api_version().await.unwrap();
    }

    #[tokio::test]
    async fn test_qb_api_torrent_info() {
        let torrent_name = "0a1d3e3ab95cf6625c266010fd13e96949ab23e7.torrent".to_string();

        let qb_task_executor = QbitTaskExecutor::new_with_login(
            "admin".to_string(), 
            "adminadmin".to_string())
            .await
            .unwrap();

        qb_task_executor.qb_api_torrent_info(torrent_name).await.unwrap();
    }

    #[tokio::test]
    async fn test_qb_api_add_torrent() {
        let qb_task_executor = QbitTaskExecutor::new_with_login(
            "admin".to_string(), 
            "adminadmin".to_string())
            .await
            .unwrap();
        

        let anime_name = "test".to_string();
        let episode_info =  EpisodeInfo { 
                mikan_id: 3144, 
                episode: 1, 
                subgroup_name: "tttttt12345".to_string(), 
                seed_name: "[Airota][Kanojo mo Kanojo][09][WebRip AVC-8bit 1080p AAC][CHS].mp4".to_string(), 
                seed_url: "/Download/20231014/0a1d3e3ab95cf6625c266010fd13e96949ab23e7.torrent".to_string() };
        
        qb_task_executor.qb_api_add_torrent(&anime_name, &episode_info).await.unwrap();
    }
}