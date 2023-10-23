use actix_web::web;
use anyhow::Result;
use diesel::{RunQueryDsl, delete};
use diesel::dsl::insert_into;
use diesel::prelude::*;
use crate::Pool;
use crate::models::anime_broadcast::*;
use crate::schema::anime_broadcast::dsl::*;

// insert single data into anime_broadcast
#[allow(dead_code)]
pub async fn add(
    pool: web::Data<Pool>,
    item: AnimeBroadcastJson
) -> Result<AnimeBroadcast, diesel::result::Error> {
    let db_connection = pool.get().unwrap();
    match anime_broadcast.filter(mikan_id.eq(&item.mikan_id)).first::<AnimeBroadcast>(&db_connection) {
        Ok(result) => Ok(result),
        Err(_) => {
            let new_anime_broadcast = PostAnimeBroadcast{
                mikan_id : &item.mikan_id,
                year     : &item.year,
                season   : &item.season
            };
            insert_into(anime_broadcast)
                .values(&new_anime_broadcast)
                .execute(&db_connection)
                .expect("Error saving new anime");
            let result = anime_broadcast.order(id.desc())
                .first(&db_connection).unwrap(); 
            Ok(result)
        }
    }
}

// insert Vec<data> into anime_broadcast
pub async fn add_vec(
    pool: web::Data<Pool>,
    item_vec: Vec<AnimeBroadcastJson>
) -> Result<i32, diesel::result::Error> {
    use crate::schema::anime_broadcast::dsl::*;
    let db_connection = pool.get().unwrap();
    let mut sucess_num: i32 = 0;

    for item in &item_vec {
        if let Err(_) = anime_broadcast.filter(mikan_id.eq(&item.mikan_id)).first::<AnimeBroadcast>(&db_connection) {
            let new_anime_broadcast = PostAnimeBroadcast{
                mikan_id : &item.mikan_id,
                year     : &item.year,
                season   : &item.season,
            };
            insert_into(anime_broadcast)
                .values(&new_anime_broadcast)
                .execute(&db_connection)
                .expect("save failed");
            sucess_num += 1;
        }
    }
    Ok(sucess_num)
}

// query all data from anime_broadcast
pub async fn get_all(
    pool: web::Data<Pool>
) -> Result<Vec<AnimeBroadcast>, diesel::result::Error> {
    let db_connection = pool.get().unwrap();
    let result: Vec<AnimeBroadcast> = anime_broadcast.load::<AnimeBroadcast>(&db_connection)?;
    Ok(result)
}