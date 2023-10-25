use serde::{Deserialize, Serialize};
use crate::schema::*;

#[derive(Debug, Serialize, Deserialize, Queryable)]
pub struct AnimeSubgroup {
    pub id: Option<i32>,
    pub subgroup_id: i32,
    pub subgroup_name: String,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = anime_subgroup)]
pub struct PostAnimeSubgroup<'a> {
    pub subgroup_id: &'a i32,
    pub subgroup_name: &'a str,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnimeSubgroupJson {
    pub subgroup_id: i32,
    pub subgroup_name: String,
}