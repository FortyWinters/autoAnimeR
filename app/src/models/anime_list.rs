use serde::{Deserialize, Serialize};
use crate::schema::*;

#[derive(Debug, Serialize, Deserialize, Queryable, Eq, PartialEq)]
pub struct AnimeList {
    pub id: Option<i32>,
    pub mikan_id: i32,
    pub anime_name: String,
    pub update_day: i32,
    pub img_url: String,
    pub anime_type: i32,
    pub subscribe_status: i32
}

impl Ord for AnimeList {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.subscribe_status != other.subscribe_status {
            other.subscribe_status.cmp(&self.subscribe_status)
        } else {
            self.update_day.cmp(&other.update_day)
        }
    }
}

impl PartialOrd for AnimeList {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Insertable)]
#[diesel(table_name = anime_list)]
pub struct PostAnimeList<'a> {
    pub mikan_id: &'a i32,
    pub anime_name: &'a str,
    pub update_day: &'a i32,
    pub img_url: &'a str,
    pub anime_type: &'a i32,
    pub subscribe_status: &'a i32
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnimeListJson {
    pub mikan_id: i32,
    pub anime_name: String,
    pub update_day: i32,
    pub img_url: String,
    pub anime_type: i32,
    pub subscribe_status: i32
}