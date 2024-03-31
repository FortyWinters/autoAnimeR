use crate::dao;
use crate::models::anime_list;
use crate::Pool;
use actix_web::{get, web, Error, HttpResponse};
use anyhow::Result;
use diesel::r2d2::{ConnectionManager, PooledConnection};
use diesel::SqliteConnection;
use log;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[get("/home")]
pub async fn home_handler(pool: web::Data<Pool>) -> Result<HttpResponse, Error> {
    let db_connection = &mut pool.get().unwrap();
    log::info!("[API] anime_index");
    Ok(match my_anime(db_connection).await {
        Ok(res) => HttpResponse::Ok().json(res),
        _ => HttpResponse::from(HttpResponse::InternalServerError()),
    })
}

pub async fn my_anime(
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
) -> Result<Vec<anime_list::AnimeList>, Error> {
    let mut anime_vec = dao::anime_list::get_by_subscribestatus(db_connection, 1)
        .await
        .unwrap();
    let task_vec = dao::anime_task::get_all(db_connection).await.unwrap();
    let mut task_mikan_id_set: HashSet<i32> = HashSet::new();
    for task in task_vec {
        if !task_mikan_id_set.contains(&task.mikan_id) {
            task_mikan_id_set.insert(task.mikan_id);
            if let Ok(anime) = dao::anime_list::get_by_mikanid(db_connection, task.mikan_id).await {
                if anime.subscribe_status == 0 {
                    task_mikan_id_set.insert(anime.mikan_id);
                    anime_vec.push(anime);
                }
            } else {
                println!("this anime is not in db, mikan_id: {}", task.mikan_id)
            }
        }
    }

    for anime in anime_vec.iter_mut() {
        let mut parts = anime.img_url.split('/');
        let img_name = parts.nth(4).unwrap();
        anime.img_url = format!("/static/img/anime_list/{}", img_name);
    }
    anime_vec.sort();
    Ok(anime_vec)
}
