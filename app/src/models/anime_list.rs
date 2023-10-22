use serde::{Deserialize, Serialize};
use crate::schema::*;

#[derive(Debug, Serialize, Deserialize, Queryable)]
pub struct AnimeList {
    pub id: Option<i32>,
    pub mikan_id: i32,
    pub anime_name: String,
    pub update_day: i32,
    pub img_url: String,
    pub anime_type: i32,
    pub subscribe_status: i32
}

#[derive(Debug, Insertable)]
#[table_name = "anime_list"]
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