use serde::{Deserialize, Serialize};
use crate::schema::*;

#[derive(Debug, Serialize, Deserialize, Queryable)]
pub struct AnimeTask {
    pub id: Option<i32>,
    pub mikan_id: i32,
    pub episode: i32,
    pub torrent_name: String,
    pub qb_task_status: i32,
    pub rename_status: i32,
    pub filename: String
}

#[derive(Debug, Insertable)]
#[diesel(table_name = anime_task)]
pub struct PostAnimeTask<'a> {
    pub mikan_id: &'a i32,
    pub episode: &'a i32,
    pub torrent_name: &'a str,
    pub qb_task_status: &'a i32,
    pub rename_status: &'a i32,
    pub filename: &'a String
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnimeTaskJson {
    pub mikan_id: i32,
    pub episode: i32,
    pub torrent_name: String,
    pub qb_task_status: i32,
    pub rename_status: i32,
    pub filename: String
}