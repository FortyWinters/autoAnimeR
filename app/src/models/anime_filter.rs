use serde::{Deserialize, Serialize};
use crate::schema::*;

#[derive(Debug, Serialize, Deserialize, Queryable, Clone)]
pub struct AnimeFilter {
    pub id: Option<i32>,
    pub mikan_id: i32,
    pub fiter_type: String,
    pub filter_val: i32,
    pub object: i32
}

#[derive(Debug, Insertable)]
#[diesel(table_name = anime_filter)]
pub struct PostAnimeFilter<'a> {
    pub mikan_id: &'a i32,
    pub filter_type: &'a str,
    pub filter_val: &'a i32,
    pub object: &'a i32
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnimeFilterJson {
    pub mikan_id: i32,
    pub fiter_type: String,
    pub filter_val: i32,
    pub object: i32
} 