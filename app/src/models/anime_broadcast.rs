use serde::{Deserialize, Serialize};
use crate::schema::*;

#[derive(Debug, Serialize, Deserialize, Queryable)]
pub struct AnimeBroadcast {
    pub id: Option<i32>,
    pub mikan_id: i32,
    pub year: i32,
    pub season: i32
}

#[derive(Debug, Insertable)]
#[table_name = "anime_broadcast"]
pub struct PostAnimeBroadcast<'a> {
    pub mikan_id: &'a i32,
    pub year: &'a i32,
    pub season: &'a i32
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnimeBroadcastJson {
    pub mikan_id: i32,
    pub year: i32,
    pub season: i32
}