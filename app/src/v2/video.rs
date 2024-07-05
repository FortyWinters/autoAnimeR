use crate::api::do_anime_task::{self, VideoConfig};
use crate::models::anime_task::AnimeTask;
use crate::mods::qb_api::QbitTaskExecutor;
use crate::mods::video_proccessor;
use crate::{dao, Pool};
use actix_web::{post, web, Error, HttpResponse};
use anyhow::Result;
use diesel::r2d2::{ConnectionManager, PooledConnection};
use diesel::SqliteConnection;
use serde::{Deserialize, Serialize};
use std::io::{Seek, Write};
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
    qb: web::Data<Arc<TokioRwLock<QbitTaskExecutor>>>,
) -> Result<HttpResponse, Error> {
    let subtitles = get_subtitle_path(&item.video_name, video_file_lock, qb)
        .await
        .map_err(|e| handle_error(e, "get_subtitle_path_handler failed"))?;

    log::info!("subtitles: {:?}", subtitles);

    let res = if subtitles.len() > 1 {
        subtitles[0].clone()
    } else {
        String::new()
    };

    Ok(HttpResponse::Ok().json(res))
}

async fn get_subtitle_path(
    video_name: &String,
    video_file_lock: web::Data<Arc<TokioRwLock<bool>>>,
    qb: web::Data<Arc<TokioRwLock<QbitTaskExecutor>>>,
) -> Result<Vec<String>, Error> {
    let _guard = video_file_lock.read().await;
    let qb = qb.read().await;
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

#[post("/extract_subtitle")]
pub async fn extract_subtitle_handle(
    item: web::Json<VideoInfoRequestJson>,
    video_file_lock: web::Data<Arc<TokioRwLock<bool>>>,
    qb: web::Data<Arc<TokioRwLock<QbitTaskExecutor>>>,
    pool: web::Data<Pool>,
) -> Result<HttpResponse, Error> {
    let db_connection = &mut pool
        .get()
        .map_err(|e| handle_error(e, "failed to get db connection"))?;

    extract_subtitle(&item.torrent_name, video_file_lock, qb, db_connection)
        .await
        .map_err(|e| handle_error(e, "Failed to extract subtitle"))?;

    Ok(HttpResponse::Ok().json("ok"))
}

pub async fn extract_subtitle(
    torrent_name: &String,
    video_file_lock: web::Data<Arc<TokioRwLock<bool>>>,
    qb: web::Data<Arc<TokioRwLock<QbitTaskExecutor>>>,
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
) -> Result<(), Error> {
    let anime_task = dao::anime_task::get_by_torrent_name(db_connection, torrent_name)
        .await
        .map_err(|e| handle_error(e, "dao::anime_task::get_by_torrent_name failed"))?;

    if anime_task.qb_task_status == 0 {
        let nb_new_finished_task = do_anime_task::auto_update_handler(&qb, db_connection)
            .await
            .map_err(|e| handle_error(e, "Failed to get finished task"))?;

        if nb_new_finished_task > 0 {
            do_anime_task::auto_rename_and_extract_handler(&video_file_lock, &qb, db_connection)
                .await
                .map_err(|e| handle_error(e, "Failed to get finished task"))?;
        }
        return Ok(());
    }

    if anime_task.rename_status == 0 {
        do_anime_task::auto_rename_and_extract_handler(&video_file_lock, &qb, db_connection)
            .await
            .map_err(|e| handle_error(e, "Failed to execute rename task"))?;
        return Ok(());
    }

    let qb = qb.read().await;
    let download_path = qb.qb_api_get_download_path().await.unwrap();
    let anime_name = dao::anime_list::get_by_mikanid(db_connection, anime_task.mikan_id)
        .await
        .map_err(|e| handle_error(e, "Failed to get anime name."))?
        .anime_name;

    let cur_total_file_path = format!(
        "{}/{}({})/{}",
        download_path, anime_name, anime_task.mikan_id, anime_task.filename
    );

    let mut subtitle_vec: Vec<String> = vec![];
    let extension = anime_task.filename.split(".").last().unwrap();
    if extension == "mkv" || extension == "mp4" {
        if let Ok(res) = video_proccessor::extract_subtitle(&cur_total_file_path).await {
            subtitle_vec = res;
        } else {
            log::warn!("");
        }
    }

    // write VideoConfig
    let cur_config = VideoConfig {
        torrent_hash: anime_task.torrent_name.clone(),
        mikan_id: anime_task.mikan_id,
        episode: anime_task.episode,
        subtitle_nb: subtitle_vec.len() as i32,
        subtitle: subtitle_vec,
    };

    let _ = video_file_lock.write().await;

    let (mut file, mut video_config) = do_anime_task::get_video_config(&download_path)
        .await
        .map_err(|e| handle_error(e, "Failed to get video config"))?;

    video_config.insert(anime_task.filename.clone(), cur_config);

    file.seek(std::io::SeekFrom::Start(0)).unwrap();
    file.set_len(0).unwrap();
    file.write_all(
        serde_json::to_string_pretty(&video_config)
            .unwrap()
            .as_bytes(),
    )
    .map_err(|e| handle_error(e, "Failed to update video config file."))?;

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::Config;
    use crate::Pool;

    #[tokio::test]
    pub async fn test() {
        dotenv::dotenv().ok();

        let config = Config::load_config("./config/config.yaml").await.unwrap();
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let database_pool = Pool::builder()
            .build(ConnectionManager::<SqliteConnection>::new(database_url))
            .expect("Failed to create pool.");

        let qb = Arc::new(TokioRwLock::new(
            QbitTaskExecutor::new_with_config(&config)
                .await
                .expect("Failed to create qb client"),
        ));

        let db_connection = &mut database_pool.get().unwrap();

        let video_file_lock = Arc::new(TokioRwLock::new(false));

        extract_subtitle(
            &"da884080bade6f74ef533cba97dd6c125773ac40.torrent".to_string(),
            web::Data::new(video_file_lock),
            web::Data::new(qb),
            db_connection,
        )
        .await
        .unwrap();
    }
}
