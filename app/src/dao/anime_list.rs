use crate::{Pool, models::anime_list::AnimeListJson};
use actix_web::web;
use crate::models::anime_list::{AnimeList, PostAnimeList};
use anyhow::Result;
use diesel::{RunQueryDsl, delete};
use diesel::dsl::insert_into;
use diesel::prelude::*;

pub async fn add_single_anime_list(
    pool: web::Data<Pool>,
    item: web::Json<AnimeListJson>
) -> Result<AnimeList, diesel::result::Error> {
    use crate::schema::anime_list::dsl::*;
    let db_connection = pool.get().unwrap();
    match anime_list
        .filter(anime_name.eq(&item.anime_name))
        .first::<AnimeList>(&db_connection) {
            Ok(result) => Ok(result),
            Err(_) => {
                let new_anime_list = PostAnimeList{
                    mikan_id: &item.mikan_id,
                    anime_name: &item.anime_name,
                    img_url: &item.img_url,
                    update_day: &item.update_day,
                    anime_type: &item.anime_type,
                    subscribe_status: &item.subscribe_status
                };
                insert_into(anime_list)
                    .values(&new_anime_list)
                    .execute(&db_connection)
                    .expect("Error saving new anime");
                let result = anime_list.order(id.desc())
                    .first(&db_connection).unwrap(); 
                Ok(result)
            }
        }
}

pub async fn get_all(
    pool: web::Data<Pool>
) -> Result<Vec<AnimeList>, diesel::result::Error> {
    use crate::schema::anime_list::dsl::*;
    let db_connection = pool.get().unwrap();
    let result: Vec<AnimeList> = anime_list.load::<AnimeList>(&db_connection)?;
    Ok(result)
}

pub async fn del(
    pool: web::Data<Pool>,
    path: web::Path<String>
) -> Result<usize, diesel::result::Error> {
    use crate::schema::anime_list::dsl::*;
    let db_connection = pool.get().unwrap();
    let id_string = path.into_inner();
    let i: i32 = id_string.parse().unwrap();
    let result = delete(anime_list.filter(mikan_id.eq(i))).execute(&db_connection)?;
    Ok(result)
}