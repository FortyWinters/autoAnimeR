use crate::api::do_anime_task;
use crate::models::anime_task::AnimeTask;
use crate::mods::qb_api::QbitTaskExecutor;
use crate::{dao, Pool};
use actix_web::{post, web, Error, HttpResponse};
use anyhow::Result;
use diesel::r2d2::{ConnectionManager, PooledConnection};
use diesel::SqliteConnection;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock as TokioRwLock;

pub fn handle_error<E: std::fmt::Debug>(e: E, message: &str) -> actix_web::Error {
    log::error!("{}, error: {:?}", message, e);
    actix_web::error::ErrorInternalServerError("Internal server error")
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VideoInfoRequestJson {
    pub torrent_name: String,
}

#[post("/get_anime_task")]
pub async fn get_anime_task_handler(
    item: web::Json<VideoInfoRequestJson>,
    pool: web::Data<Pool>,
) -> Result<HttpResponse, Error> {
    let db_connection = &mut pool
        .get()
        .map_err(|e| handle_error(e, "failed to get db connection"))?;
    let res = get_anime_task(&item.torrent_name, db_connection)
        .await
        .unwrap();
    Ok(HttpResponse::Ok().json(res))
}

async fn get_anime_task(
    torrent_name: &String,
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
) -> Result<AnimeTask, Error> {
    let res = dao::anime_task::get_by_torrent_name(db_connection, torrent_name)
        .await
        .map_err(|e| handle_error(e, "dao::anime_task::get_by_torrent_name failed"))?;

    Ok(res)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VideoName {
    pub video_name: String,
}
#[post("/get_subtitle_path")]
pub async fn get_subtitle_path_handler(
    item: web::Json<VideoName>,
    video_file_lock: web::Data<Arc<TokioRwLock<bool>>>,
    qb: web::Data<QbitTaskExecutor>,
) -> Result<HttpResponse, Error> {
    let res = get_subtitle_path(&item.video_name, video_file_lock, qb)
        .await
        .map_err(|e| handle_error(e, "get_subtitle_path_handler failed"))?;
    log::info!("subtitles: {:?}", res);
    Ok(HttpResponse::Ok().json(res[0].clone()))
}

async fn get_subtitle_path(
    video_name: &String,
    video_file_lock: web::Data<Arc<TokioRwLock<bool>>>,
    qb: web::Data<QbitTaskExecutor>,
) -> Result<Vec<String>, Error> {
    let _guard = video_file_lock.read().await;
    let download_path = qb.qb_api_get_download_path().await.unwrap();

    let (_, video_config) = do_anime_task::get_video_config(&download_path)
        .await
        .map_err(|e| handle_error(e, "Failed to get video config"))?;

    let mut subtitle_vec: Vec<String> = vec![];
    if let Ok(cfg) = video_config.get(video_name).ok_or("error") {
        for subtitle in &cfg.subtitle {
            subtitle_vec.push(subtitle.clone());
        }
    } else {
        log::warn!("Failed to get subtitle, video name: {:?}", video_name);
    }
    Ok(subtitle_vec)
}