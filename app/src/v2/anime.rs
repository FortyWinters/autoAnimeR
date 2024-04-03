use crate::dao;
use crate::models::{anime_broadcast, anime_list};
use crate::Pool;
use actix_web::{get, post, web, Error, HttpResponse};
use anyhow::Result;
use diesel::r2d2::{ConnectionManager, PooledConnection};
use diesel::SqliteConnection;
use log;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

fn handle_error<E: std::fmt::Debug>(e: E, message: &str) -> actix_web::Error {
    log::error!("{}, error: {:?}", message, e);
    actix_web::error::ErrorInternalServerError("Internal server error")
}

fn get_img_name_from_url(img_url: &str) -> Option<String> {
    let parts: Vec<&str> = img_url.split('/').collect();
    if let Some(img_name) = parts.get(4) {
        Some(img_name.to_string())
    } else {
        log::warn!("unexpected img_url format: {}", img_url);
        None
    }
}

#[get("/home")]
pub async fn get_anime_home_handler(pool: web::Data<Pool>) -> Result<HttpResponse, Error> {
    log::info!("[API][V2][ANIME] get_anime_home_handler: /home");
    let db_connection = &mut pool
        .get()
        .map_err(|e| handle_error(e, "get_anime_home_handler, failed to get db connection"))?;

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
        .map_err(|e| {
            handle_error(
                e,
                "get_anime_home, dao::anime_list::get_by_subscribestatus failed",
            )
        })?;

    let task_vec = dao::anime_task::get_all(db_connection)
        .await
        .map_err(|e| handle_error(e, "get_anime_home, dao::anime_task::get_all failed"))?;

    let mut task_mikan_id_set: HashSet<i32> = HashSet::new();
    for task in task_vec {
        if !task_mikan_id_set.insert(task.mikan_id) {
            continue;
        }
        if let Ok(anime) = dao::anime_list::get_by_mikanid(db_connection, task.mikan_id)
            .await
            .map_err(|e| handle_error(e, "get_anime_home, dao::anime_list::get_by_mikanid failed"))
        {
            if anime.subscribe_status == 0 {
                anime_vec.push(anime);
            }
        }
    }

    for anime in anime_vec.iter_mut() {
        anime.img_url =
            get_img_name_from_url(&anime.img_url).unwrap_or_else(|| anime.img_url.clone());
    }
    anime_vec.sort();
    Ok(anime_vec)
}

#[get("/info/{mikan_id}")]
pub async fn get_anime_info_handler(
    pool: web::Data<Pool>,
    path: web::Path<(i32,)>,
) -> Result<HttpResponse, Error> {
    log::info!("[API][V2][ANIME] get_anime_info_handler: /info/{}", path.0);
    let db_connection = &mut pool
        .get()
        .map_err(|e| handle_error(e, "get_anime_info_handler, failed to get db connection"))?;

    let res = get_anime_info(db_connection, path.0)
        .await
        .map_err(|e| handle_error(e, "get_anime_info_handler, get_anime_info failed"))?;

    Ok(HttpResponse::Ok().json(res))
}

async fn get_anime_info(
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
    mikan_id: i32,
) -> Result<anime_list::AnimeList, Error> {
    let mut anime_info = dao::anime_list::get_by_mikanid(db_connection, mikan_id)
        .await
        .map_err(|e| handle_error(e, "get_anime_info, dao::anime_list::get_by_mikanid failed"))?;

    anime_info.img_url =
        get_img_name_from_url(&anime_info.img_url).unwrap_or_else(|| anime_info.img_url.clone());
    Ok(anime_info)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnimeRequestJson {
    pub mikan_id: i32,
    pub subscribe_status: i32,
}

#[post("/subscribe")]
pub async fn subscribe_anime_handler(
    pool: web::Data<Pool>,
    item: web::Json<AnimeRequestJson>,
) -> Result<HttpResponse, Error> {
    log::info!(
        "[API][V2][ANIME] subscribe_anime_handler: /subscribe {:?}",
        item
    );

    let db_connection = &mut pool
        .get()
        .map_err(|e| handle_error(e, "subscribe_anime_handler, failed to get db connection"))?;

    let res = subscribe_anime(db_connection, item.into_inner())
        .await
        .map_err(|e| handle_error(e, "subscribe_anime_handler, subscribe_anime failed"))?;

    Ok(HttpResponse::Created().json(res))
}

async fn subscribe_anime(
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
    item: AnimeRequestJson,
) -> Result<(), Error> {
    let mikan_id = item.mikan_id;
    let subscribe_status = if item.subscribe_status == 1 { 0 } else { 1 };

    dao::anime_list::update_subscribestatus_by_mikanid(db_connection, mikan_id, subscribe_status)
        .await
        .map_err(|e| {
            handle_error(
                e,
                "subscribe_anime, dao::anime_list::update_subscribestatus_by_mikanid failed",
            )
        })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BroadcastRequestJson {
    pub year: i32,
    pub season: i32,
}

#[get("/broadcast/{year}/{season}")]
pub async fn get_anime_broadcast_handler(
    pool: web::Data<Pool>,
    path: web::Path<(i32, i32)>,
) -> Result<HttpResponse, Error> {
    let (year, season) = path.into_inner();
    log::info!(
        "[API][V2][ANIME] get_anime_broadcast_handler: /broadcast/{}/{}",
        year,
        season
    );

    let db_connection = &mut pool.get().map_err(|e| {
        handle_error(
            e,
            "get_anime_broadcast_handler, failed to get db connection",
        )
    })?;

    let res = get_anime_broadcast(db_connection, year, season)
        .await
        .map_err(|e| handle_error(e, "get_anime_broadcast_handler, get_anime_broadcast failed"))?;

    Ok(HttpResponse::Ok().json(res))
}

async fn get_anime_broadcast(
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
    year: i32,
    season: i32,
) -> Result<Vec<anime_list::AnimeList>, Error> {
    let broadcast_list: Vec<anime_broadcast::AnimeBroadcast> =
        dao::anime_broadcast::get_by_year_season(db_connection, year, season)
            .await
            .map_err(|e| {
                handle_error(
                    e,
                    "get_anime_broadcast, dao::anime_broadcast::get_by_year_season failed",
                )
            })?;

    let mut anime_vec: Vec<anime_list::AnimeList> = Vec::new();

    for anime_broadcast in &broadcast_list {
        let mut anime = dao::anime_list::get_by_mikanid(db_connection, anime_broadcast.mikan_id)
            .await
            .map_err(|e| {
                handle_error(
                    e,
                    "get_anime_broadcast, dao::anime_list::get_by_mikanid failed",
                )
            })?;

        anime.img_url =
            get_img_name_from_url(&anime.img_url).unwrap_or_else(|| anime.img_url.clone());

        anime_vec.push(anime);
    }
    anime_vec.sort();
    Ok(anime_vec)
}
