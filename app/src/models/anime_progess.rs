use serde::{Deserialize, Serialize};
use crate::schema::*;

#[derive(Debug, Serialize, Deserialize, Queryable, Insertable)]
#[diesel(table_name = anime_progress)]
pub struct AnimeProgress {
    pub id: Option<i32>,
    pub progress_id: String,
    pub mikan_id: i32,
    pub episode: i32,
    pub torrent_name: String,
    pub progress_status: i32,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = anime_progress)]
pub struct PostAnimeProgress<'a> {
    pub progress_id: &'a str,
    pub mikan_id: &'a i32,
    pub episode: &'a i32,
    pub torrent_name: &'a str,
    pub progress_status: &'a i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnimeProgressJson {
    pub progress_id: String,
    pub mikan_id: i32,
    pub episode: i32,
    pub torrent_name: String,
    pub progress_status: i32,
}