use crate::api::do_anime_task::{self, VideoConfig};
use crate::models::anime_progess::AnimeProgressJson;
use crate::mods::config::Config;
use crate::mods::qb_api::QbitTaskExecutor;
use crate::mods::video_proccessor::{self, get_av_hwaccels, trans_mkv_2_mp4};
use crate::{api, dao, Pool};
use actix_web::{get, post, web, Error, HttpResponse};
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
pub struct TorrentName {
    pub torrent_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VideoDetail {
    pub anime_name: String,
    pub subgroup_name: String,
    pub video_path: String,
    pub subtitle_vec: Vec<String>,
}

#[post("/get_video_detail")]
pub async fn get_video_detail_handler(
    item: web::Json<TorrentName>,
    video_file_lock: web::Data<Arc<TokioRwLock<bool>>>,
    qb: web::Data<Arc<TokioRwLock<QbitTaskExecutor>>>,
    pool: web::Data<Pool>,
) -> Result<HttpResponse, Error> {
    let db_connection = &mut pool
        .get()
        .map_err(|e| handle_error(e, "failed to get db connection"))?;
    match get_anime_detail(&item.torrent_name, video_file_lock, qb, db_connection).await {
        Ok(res) => Ok(HttpResponse::Ok().json(res)),
        Err(e) => Err(Error::from(e)),
    }
}

async fn get_anime_detail(
    torrent_name: &String,
    video_file_lock: web::Data<Arc<TokioRwLock<bool>>>,
    qb: web::Data<Arc<TokioRwLock<QbitTaskExecutor>>>,
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
) -> Result<VideoDetail, Error> {
    let anime_task = dao::anime_task::get_by_torrent_name(db_connection, torrent_name)
        .await
        .map_err(|e| handle_error(e, "dao::anime_task::get_by_torrent_name failed"))?;

    let subgroup_id = dao::anime_seed::get_anime_seed_by_seed_url(db_connection, torrent_name)
        .await
        .map_err(|e| handle_error(e, "dao::anime_task::get_by_torrent_name failed"))?
        .subgroup_id;

    let anime_name = dao::anime_list::get_by_mikanid(db_connection, anime_task.mikan_id)
        .await
        .map_err(|e| handle_error(e, "dao::anime_task::get_by_torrent_name failed"))?
        .anime_name;

    let subgroup_name = dao::anime_subgroup::get_by_subgroupid(db_connection, &subgroup_id)
        .await
        .map_err(|e| handle_error(e, "dao::anime_task::get_by_torrent_name failed"))?
        .subgroup_name;

    let file_path = format!("{}({})", anime_name, anime_task.mikan_id);

    let video_path = format!("{}/{}", file_path, anime_task.filename);

    let subtitle_vec = match get_subtitle_vec(&anime_task.filename, video_file_lock, qb).await {
        Ok(res) => res
            .into_iter()
            .map(|s| format!("{}/{}", file_path, s))
            .collect(),
        Err(_) => {
            log::warn!("Failed to get subtitle for torrent: {}", torrent_name);
            Vec::new()
        }
    };

    Ok(VideoDetail {
        anime_name,
        subgroup_name,
        video_path,
        subtitle_vec,
    })
}

async fn get_subtitle_vec(
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
    item: web::Json<TorrentName>,
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
        torrent_name: anime_task.torrent_name.clone(),
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

#[derive(Debug, Serialize, Deserialize)]
pub struct ReqAnimeProgress {
    pub progress_id: String,
    pub mikan_id: Option<i32>,
    pub episode: Option<i32>,
    pub torrent_name: Option<String>,
    pub progress_status: Option<i32>,
}

impl ReqAnimeProgress {
    pub fn to_anime_progress_json(&self) -> AnimeProgressJson {
        AnimeProgressJson {
            progress_id: self.progress_id.clone(),
            mikan_id: self.mikan_id.unwrap_or(-1),
            episode: self.episode.unwrap_or(-1),
            torrent_name: self.torrent_name.clone().unwrap_or("".to_string()),
            progress_status: self.progress_status.unwrap_or(0),
        }
    }
}

#[post("/get_anime_progress")]
pub async fn get_anime_progress_handler(
    item: web::Json<ReqAnimeProgress>,
    pool: web::Data<Pool>,
) -> Result<HttpResponse, Error> {
    let res = match (&item.mikan_id, &item.episode, &item.torrent_name) {
        (Some(mikan_id), Some(episode), _) => {
            get_anime_progress_by_mikanid_and_episode(&item.progress_id, mikan_id, episode, pool)
                .await
        }
        (_, _, Some(torrent_name)) => {
            get_anime_progress_by_torrent(&item.progress_id, torrent_name, pool).await
        }
        _ => Ok(0),
    };

    Ok(HttpResponse::Ok().json(res.unwrap_or(0)))
}

async fn get_anime_progress_by_mikanid_and_episode(
    progress_id: &String,
    mikan_id: &i32,
    episode: &i32,
    pool: web::Data<Pool>,
) -> Result<i32, Error> {
    let db_connection = &mut pool
        .get()
        .map_err(|e| handle_error(e, "failed to get db connection"))?;

    match dao::anime_progress::get_by_mikan_id_and_episode(
        &progress_id,
        &mikan_id,
        &episode,
        db_connection,
    )
    .await
    {
        Ok(p) => Ok(p.progress_status),
        Err(e) => {
            log::info!(
                "Failed to get anime progress {} by [mikan_id: {}, episode: {}], {}",
                progress_id,
                mikan_id,
                episode,
                e
            );
            Ok(0)
        }
    }
}

async fn get_anime_progress_by_torrent(
    progress_id: &String,
    torrent_name: &String,
    pool: web::Data<Pool>,
) -> Result<i32, Error> {
    let db_connection = &mut pool
        .get()
        .map_err(|e| handle_error(e, "failed to get db connection"))?;

    match dao::anime_progress::get_by_torrent_name(progress_id, torrent_name, db_connection).await {
        Ok(p) => Ok(p.progress_status),
        Err(e) => {
            log::info!(
                "Failed to get anime progress [{}] by [torrent: {}], {}",
                progress_id,
                torrent_name,
                e
            );
            Ok(0)
        }
    }
}

#[post("/set_anime_progress")]
pub async fn set_anime_progress_handler(
    item: web::Json<ReqAnimeProgress>,
    pool: web::Data<Pool>,
) -> Result<HttpResponse, Error> {
    let res = match (&item.mikan_id, &item.episode, &item.torrent_name) {
        (Some(_), Some(_), _) => set_anime_progress_by_mikanid_and_episode(&item, pool).await,
        (_, _, Some(_)) => set_anime_progress_by_torrent(&item, pool).await,
        _ => Ok(()),
    };

    match res {
        Ok(_) => Ok(HttpResponse::Ok().json("ok")),
        Err(_) => Ok(HttpResponse::BadRequest().body("error")),
    }
}

async fn set_anime_progress_by_mikanid_and_episode(
    item: &ReqAnimeProgress,
    pool: web::Data<Pool>,
) -> Result<(), Error> {
    let db_connection = &mut pool
        .get()
        .map_err(|e| handle_error(e, "failed to get db connection"))?;

    let quary_item = item.to_anime_progress_json();

    dao::anime_progress::add_with_mikan_id_and_episode(&quary_item, db_connection)
        .await
        .map_err(|e| {
            handle_error(
                e,
                format!(
                    "Failed to set anime progress {} by [mikan_id: {}, episode: {}] ",
                    quary_item.progress_id, quary_item.mikan_id, quary_item.episode
                )
                .as_str(),
            )
        })?;
    Ok(())
}

async fn set_anime_progress_by_torrent(
    item: &ReqAnimeProgress,
    pool: web::Data<Pool>,
) -> Result<(), Error> {
    let mut db_connection = pool
        .get()
        .map_err(|e| handle_error(e, "failed to get db connection"))?;
    let quary_item = item.to_anime_progress_json();

    dao::anime_progress::add_with_torrent_name(&quary_item, &mut db_connection)
        .await
        .map_err(|e| {
            handle_error(
                e,
                format!(
                    "Failed to set anime progress {} by [torrent_name: {}]",
                    quary_item.progress_id, quary_item.torrent_name
                )
                .as_str(),
            )
        })?;
    drop(db_connection);
    Ok(())
}

#[get("/check_hw_accels")]
async fn check_hw_accels_handler() -> Result<HttpResponse, Error> {
    match get_av_hwaccels() {
        Ok(res) => {
            let codec_name = video_proccessor::trans_hwaccels_2_codec_name(res);
            if codec_name != "h264" {
                Ok(HttpResponse::Ok().json(codec_name))
            } else {
                Ok(HttpResponse::Ok().json(""))
            }
        }
        Err(e) => {
            log::info!("Failed to get av_hwaccels_vec, Err: {}", e);
            Ok(HttpResponse::BadRequest().body("Failed to get av_hwaccels_vec"))
        }
    }
}

#[post("/trans_video_format")]
async fn trans_video_format_handler(
    item: web::Json<TorrentName>,
    pool: web::Data<Pool>,
    config: web::Data<Arc<TokioRwLock<Config>>>,
) -> Result<HttpResponse, Error> {
    let download_path = {
        let config = config.read().await;
        config.download_path.clone()
    };

    let mut db_connection = pool
        .get()
        .map_err(|e| handle_error(e, "Failed to get DB connection"))?;

    let path = api::do_anime_task::get_filepath_by_torrent_name(
        &item.torrent_name,
        &download_path,
        &mut db_connection,
    )
    .await
    .map_err(|e| {
        handle_error(
            e,
            &format!(
                "failed to get filepath by torrent_name: {}",
                item.torrent_name
            ),
        )
    })?;

    drop(db_connection);

    tokio::spawn(async move {
        match trans_mkv_2_mp4(&path).await {
            Ok(_) => log::info!("Successfully converted video: {}", path),
            Err(e) => log::error!("Failed to convert video: {}, Err: {}", path, e),
        }
    });

    Ok(HttpResponse::Ok().json("Video conversion started"))
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
        let res = get_anime_detail(
            &"cf86dfac0c05125eac6fa800f4f1ee6227e12a2e.torrent".to_string(),
            web::Data::new(video_file_lock),
            web::Data::new(qb),
            db_connection,
        )
        .await
        .unwrap();

        println!("{:?}", res);
    }
}
