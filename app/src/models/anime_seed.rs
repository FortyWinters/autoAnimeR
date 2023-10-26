use serde::{Deserialize, Serialize};
use crate::schema::*;

#[derive(Debug, Serialize, Deserialize, Queryable, Clone)]
pub struct AnimeSeed {
    pub id: Option<i32>,
    pub mikan_id: i32,
    pub subgroup_id: i32,
    pub episode: i32,
    pub seed_name: String,
    pub seed_url: String,
    pub seed_status: i32,
    pub seed_size: String
}

#[derive(Debug, Insertable)]
#[diesel(table_name = anime_seed)]
pub struct PostAnimeSeed<'a> {
    pub mikan_id: &'a i32,
    pub subgroup_id: &'a i32,
    pub episode: &'a i32,
    pub seed_name: &'a str,
    pub seed_url: &'a str,
    pub seed_status: &'a i32,
    pub seed_size: &'a str
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnimeSeedJson {
    pub mikan_id: i32,
    pub subgroup_id: i32,
    pub episode: i32,
    pub seed_name: String,
    pub seed_url: String,
    pub seed_status: i32,
    pub seed_size: String
}