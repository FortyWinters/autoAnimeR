use regex::Regex;
use reqwest::Client;
use select::document::Document;
use select::predicate::{Attr, Name, Predicate};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::time::Duration;
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone)]
pub struct Mikan {
    client: Client,
    url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Anime {
    pub mikan_id: i32,
    pub anime_name: String,
    pub img_url: String,
    pub update_day: i32,
    pub anime_type: i32,
    pub subscribe_status: i32,
}

#[derive(Debug)]
pub struct Seed {
    pub mikan_id: i32,
    pub episode: i32,
    pub seed_url: String,
    pub subgroup_id: i32,
    pub seed_name: String,
    pub seed_status: i32,
    pub seed_size: String,
}

#[derive(Debug)]
pub struct Subgroup {
    pub subgroup_id: i32,
    pub subgroup_name: String,
}

#[derive(Debug)]
pub struct Broadcast {
    pub mikan_id: i32,
    pub year: i32,
    pub season: i32,
}

#[allow(dead_code)]
impl Mikan {
    pub fn new() -> Result<Mikan, Box<dyn Error>> {
        let client = Client::builder().timeout(Duration::from_secs(10)).build()?;
        Ok(Mikan {
            client,
            url: "https://mikanani.me".to_string(),
        })
    }

    async fn request_html(&self, url: &str) -> Result<Document, Box<dyn Error>> {
        let response = self.client.get(url).send().await?;
        if !response.status().is_success() {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Request failed",
            )));
        }
        let body = response.text().await?;
        return Ok(Document::from(body.as_str()));
    }

    pub async fn get_anime(&self, year: i32, season: i32) -> Result<Vec<Anime>, Box<dyn Error>> {
        let season_str: &str;
        match season {
            1 => season_str = "%E6%98%A5", // spring
            2 => season_str = "%E5%A4%8F", // summer
            3 => season_str = "%E7%A7%8B", // autumn
            _ => season_str = "%E5%86%AC", // winter
        }
        let url = format!(
            "{}/Home/BangumiCoverFlowByDayOfWeek?year={}&seasonStr={}",
            self.url, year, season_str
        );
        let document = self.request_html(&url).await?;

        let mut anime_list: Vec<Anime> = Vec::new();
        let mut anime_name_map: HashMap<i32, String> = HashMap::new();
        for node in document.find(Name("div").and(Attr("class", "sk-bangumi"))) {
            let anime_type: i32;
            let mut update_day = node.attr("data-dayofweek").unwrap().parse::<i32>().unwrap();
            match update_day {
                7 => {
                    anime_type = 1; // movie
                    update_day = 8;
                }
                8 => {
                    anime_type = 2; // ova
                    update_day = 9;
                }
                0 => {
                    anime_type = 0;
                    update_day = 7; // udpate on sunday
                }
                _ => {
                    anime_type = 0;
                }
            }

            for n in node.find(Name("span")) {
                let img_url = n
                    .attr("data-src")
                    .unwrap()
                    .split('?')
                    .next()
                    .unwrap()
                    .to_string();
                let mikan_id = n.attr("data-bangumiid").unwrap().parse::<i32>().unwrap();
                anime_list.push(Anime {
                    mikan_id,
                    anime_name: String::new(),
                    img_url,
                    update_day,
                    anime_type,
                    subscribe_status: 0,
                });
            }

            for n in node.find(Name("a")) {
                let anime_name = n.attr("title").unwrap().to_string();
                let mikan_id = n
                    .attr("href")
                    .unwrap()
                    .split('/')
                    .last()
                    .unwrap()
                    .parse::<i32>()
                    .unwrap();
                anime_name_map.insert(mikan_id, anime_name);
            }
        }

        let mut anime_list_res: Vec<Anime> = Vec::new();
        for mut a in anime_list {
            if let Some(anime_name) = anime_name_map.get(&a.mikan_id) {
                a.anime_name = anime_name.clone();
                anime_list_res.push(a);
            }
        }
        return Ok(anime_list_res);
    }

    pub async fn get_subgroup(&self, mikan_id: i32) -> Result<Vec<Subgroup>, Box<dyn Error>> {
        let url = format!("{}/Home/Bangumi/{}", self.url, mikan_id);
        let document = self.request_html(&url).await?;
        let mut subgroup_list: Vec<Subgroup> = Vec::new();
        for node in document.find(Name("li").and(Attr("class", "leftbar-item"))) {
            let doc = node.find(Name("a"));
            for n in doc {
                let subgroup_id = n.attr("data-anchor").unwrap()[1..].parse::<i32>().unwrap();
                let subgroup_name = n.text();
                subgroup_list.push(Subgroup {
                    subgroup_id,
                    subgroup_name,
                });
            }
        }
        return Ok(subgroup_list);
    }

    pub async fn get_seed(
        &self,
        mikan_id: i32,
        subgroup_id: i32,
        anime_type: i32,
    ) -> Result<Vec<Seed>, Box<dyn Error>> {
        let url = format!(
            "{}/Home/ExpandEpisodeTable?bangumiId={}&subtitleGroupId={}&take=65",
            self.url, mikan_id, subgroup_id
        );
        let document = self.request_html(&url).await?;
        let mut seed_list: Vec<Seed> = Vec::new();
        for (i, node) in document.find(Name("tr")).enumerate() {
            if i == 0 {
                continue;
            }

            let seed_url = node
                .find(Name("a"))
                .nth(2)
                .and_then(|n| n.attr("href"))
                .map(|href| href.to_string())
                .unwrap_or_else(|| String::new());
            let seed_info = node.text();
            let parts: Vec<&str> = seed_info.trim().split('\n').collect();
            let seed_name = parts.get(0).unwrap().to_string();
            let seed_size = parts.get(1).unwrap().replace(" ", "");

            if anime_type == 0 {
                if !regex_seed_1080(&seed_name) {
                    continue;
                }
            }

            let mut seed_episode = 1;
            if anime_type == 0 {
                if let Ok(episode) = regex_seed_episode(&seed_name) {
                    seed_episode = episode;
                } else {
                    continue;
                }
            }

            seed_list.push(Seed {
                mikan_id,
                episode: seed_episode,
                seed_url,
                subgroup_id,
                seed_name: seed_name[..seed_name.len() - 15].to_string(),
                seed_status: 0,
                seed_size,
            });
        }
        return Ok(seed_list);
    }

    async fn download(
        &self,
        download_url: &str,
        save_path: &str,
        new_name: &str,
    ) -> Result<(), Box<dyn Error>> {
        // reference: https://github.com/benkay86/async-applied/blob/master/indicatif-reqwest-tokio/src/bin/indicatif-reqwest-tokio-single.rs
        if !tokio::fs::metadata(save_path).await.is_ok() {
            tokio::fs::create_dir_all(save_path).await?;
        }

        let request = self.client.get(download_url);
        let mut outfile = tokio::fs::File::create(format!("{}/{}", save_path, new_name)).await?;
        let mut download = request.send().await?;
        while let Some(chunk) = download.chunk().await? {
            outfile.write(&chunk).await?;
        }
        outfile.flush().await?;
        Ok(())
    }

    pub async fn download_img(&self, img_url: &str, save_path: &str) -> Result<(), Box<dyn Error>> {
        let download_url = format!("{}{}", self.url, img_url);
        // print!("download url: {}, img_url: {}\n", download_url, img_url);
        let mut parts = img_url.split('/');
        let new_name = parts.nth(4).unwrap();
        self.download(&download_url, save_path, new_name).await
    }

    pub async fn download_seed(
        &self,
        seed_url: &str,
        save_path: &str,
    ) -> Result<(), Box<dyn Error>> {
        let download_url = format!("{}{}", self.url, seed_url);
        let mut parts = seed_url.split('/');
        let new_name = parts.nth(3).unwrap();
        self.download(&download_url, save_path, new_name).await
    }
}

fn regex_seed_episode(seed_name: &str) -> Result<i32, Box<dyn Error>> {
    let re1 = Regex::new(r"\d{2}-\d{2}").unwrap();
    let str_list1: Vec<&str> = re1.find_iter(seed_name).map(|mat| mat.as_str()).collect();
    if !str_list1.is_empty() {
        return Ok(-1);
    }

    let re2 = Regex::new(r"\[\d{2}\]|\s\d{2}\s").unwrap();
    let str_list2: Vec<&str> = re2.find_iter(seed_name).map(|mat| mat.as_str()).collect();
    if str_list2.is_empty() {
        let re3 = Regex::new(r"\[第\d+话\]").unwrap();
        let str_list3: Vec<&str> = re3.find_iter(seed_name).map(|mat| mat.as_str()).collect();
        if str_list3.is_empty() {
            return Err("regex episode failed".into());
        } else {
            return Ok(str_list3[0][4..str_list3[0].len() - 4]
                .parse::<i32>()
                .unwrap());
        }
    }
    return Ok(str_list2[0][1..str_list2[0].len() - 1]
        .to_string()
        .parse::<i32>()
        .unwrap());
}

fn regex_seed_1080(seed_name: &str) -> bool {
    let re = Regex::new(r"1080").unwrap();
    let str_list: Vec<&str> = re.find_iter(seed_name).map(|mat| mat.as_str()).collect();
    !str_list.is_empty()
}
