use actix_web::web;
use anyhow::Result;
use diesel::{RunQueryDsl, delete};
use diesel::dsl::insert_into;
use diesel::prelude::*;
use crate::Pool;
use crate::models::anime_list::*;
use crate::schema::anime_list::dsl::*;

// insert single data into anime_list
#[allow(dead_code)]
pub async fn add(
    pool: web::Data<Pool>,
    item: AnimeListJson
) -> Result<AnimeList, diesel::result::Error> {
    let db_connection = &mut pool.get().unwrap();
    match anime_list.filter(mikan_id.eq(&item.mikan_id)).first::<AnimeList>(db_connection) {
        Ok(result) => Ok(result),
        Err(_) => {
            let new_anime_list = PostAnimeList{
                mikan_id        : &item.mikan_id,
                anime_name      : &item.anime_name,
                img_url         : &item.img_url,
                update_day      : &item.update_day,
                anime_type      : &item.anime_type,
                subscribe_status: &item.subscribe_status
            };
            insert_into(anime_list)
                .values(&new_anime_list)
                .execute(db_connection)
                .expect("Error saving new anime");
            let result = anime_list.order(id.desc())
                .first(db_connection).unwrap(); 
            Ok(result)
        }
    }
}

// insert Vec<data> into anime_list
pub async fn add_vec(
    pool: web::Data<Pool>,
    item_vec: Vec<AnimeListJson>
) -> Result<i32, diesel::result::Error> {
    use crate::schema::anime_list::dsl::*;
    let db_connection = &mut pool.get().unwrap();
    let mut sucess_num: i32 = 0;

    for item in &item_vec {
        if let Err(_) = anime_list.filter(mikan_id.eq(&item.mikan_id)).first::<AnimeList>(db_connection) {
            let new_anime_list = PostAnimeList{
                mikan_id        : &item.mikan_id,
                anime_name      : &item.anime_name,
                img_url         : &item.img_url,
                update_day      : &item.update_day,
                anime_type      : &item.anime_type,
                subscribe_status: &item.subscribe_status
            };
            insert_into(anime_list)
                .values(&new_anime_list)
                .execute(db_connection)
                .expect("save failed");
            sucess_num += 1;
        }
    }
    Ok(sucess_num)
}

pub async fn get_by_mikanid(
    pool: web::Data<Pool>,
    query_mikanid: i32,
) -> Result<AnimeList, diesel::result::Error> {
    let db_connection = &mut pool.get().unwrap();
    let result: AnimeList = anime_list
        .filter(mikan_id.eq(query_mikanid))
        .first::<AnimeList>(db_connection)?;
    Ok(result)
}

// query all data from anime_list
pub async fn get_all(
    pool: web::Data<Pool>
) -> Result<Vec<AnimeList>, diesel::result::Error> {
    let db_connection = &mut pool.get().unwrap();
    let result: Vec<AnimeList> = anime_list.load::<AnimeList>(db_connection)?;
    Ok(result)
}

// delete single data by mikan_id
#[allow(dead_code)]
pub async fn del_by_mikan_id(
    pool: web::Data<Pool>,
    i: i32, 
) -> Result<usize, diesel::result::Error> {
    let db_connection = &mut pool.get().unwrap();
    let result = delete(anime_list.filter(mikan_id.eq(i))).execute(db_connection)?;
    Ok(result)
}