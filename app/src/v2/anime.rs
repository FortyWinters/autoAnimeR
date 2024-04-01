use crate::dao;
use crate::models::anime_list;
use crate::Pool;
use actix_web::{get, web, Error, HttpResponse};
use anyhow::Result;
use diesel::r2d2::{ConnectionManager, PooledConnection};
use diesel::SqliteConnection;
use log;
use std::collections::HashSet;

fn handle_error<E: std::fmt::Debug>(e: E, message: &str) -> actix_web::Error {
    log::error!("{}, error: {:?}", message, e);
    actix_web::error::ErrorInternalServerError("Internal server error")
}

#[get("/home")]
pub async fn get_anime_home_handler(pool: web::Data<Pool>) -> Result<HttpResponse, Error> {
    let db_connection = &mut pool
        .get()
        .map_err(|e| handle_error(e, "get_anime_home_handler, failed to get db connection"))?;

    log::info!("[API][V2][ANIME] get_anime_home_handler: /home");

    let res = get_anime_home(db_connection)
        .await
        .map_err(|e| handle_error(e, "get_anime_home_handler, get_anime_home failed"))?;

    Ok(HttpResponse::Ok().json(res))
}

async fn get_anime_home(
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
) -> Result<Vec<anime_list::AnimeList>, Error> {
    let mut anime_vec = dao::anime_list::get_by_subscribestatus(db_connection, 1)
        .await
        .map_err(|e| handle_error(e, "get_anime_home, dao::anime_list::get_by_subscribestatus"))?;

    let task_vec = dao::anime_task::get_all(db_connection)
        .await
        .map_err(|e| handle_error(e, "get_anime_home, dao::anime_task::get_all"))?;

    let mut task_mikan_id_set: HashSet<i32> = HashSet::new();
    for task in task_vec {
        if !task_mikan_id_set.insert(task.mikan_id) {
            continue;
        }
        if let Ok(anime) = dao::anime_list::get_by_mikanid(db_connection, task.mikan_id)
            .await
            .map_err(|e| handle_error(e, "get_anime_home, dao::anime_list::get_by_mikanid"))
        {
            if anime.subscribe_status == 0 {
                anime_vec.push(anime);
            }
        }
    }

    for anime in anime_vec.iter_mut() {
        let parts: Vec<&str> = anime.img_url.split('/').collect();
        if let Some(img_name) = parts.get(4) {
            // anime.img_url = format!("/static/img/anime_list/{}", img_name);
            anime.img_url = img_name.to_string();
        } else {
            log::warn!(
                "get_anime_home, unexpected img_url format: {}",
                anime.img_url
            )
        }
    }

    anime_vec.sort();
    Ok(anime_vec)
}
