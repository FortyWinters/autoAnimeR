use serde::{Deserialize, Serialize};
use crate::schema::*;

#[derive(Debug, Serialize, Deserialize, Queryable)]
pub struct AnimeSeed {
    pub mikan_id: i32,
    pub subgroup_id: i32,
    pub episode: i32,
    pub seed_name: String,
    pub seed_url: String,
    pub seed_status: i32,
    pub seed_size: String,
}

#[derive(Debug, Insertable)]
#[table_name = "anime_seed"]
pub struct PostAnimeList<'a> {
    pub mikan_id: &'a i32,
    pub subgroup_id: &'a i32,
    pub episode: &'a i32,
    pub seed_name: &'a str,
    pub seed_url: &'a str,
    pub seed_status: &'a i32,
    pub seed_size: &'a str
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