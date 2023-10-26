use actix_web::{post, get, web, HttpResponse, Error};
use anyhow::Result;
use tera::Context;
use serde::{Deserialize, Serialize};
use diesel::r2d2::{PooledConnection, ConnectionManager};
use diesel::SqliteConnection;
use crate::api::anime::{BroadcastUrl, get_broadcast_map};
use crate::mods::qb_api::{QbitTaskExecutor, TorrentInfo};
use crate::Pool;
use crate::dao;
use crate::api::anime::get_anime_id_name_map;


#[get("/")]
pub async fn download_index_handler(
    tera: web::Data<tera::Tera>
) -> Result<HttpResponse, Error> {
    Ok(
        match download_index(tera)
            .await {
                Ok(res) => res,
                _ => HttpResponse::from(HttpResponse::InternalServerError()),
            },
    )
}

pub async fn download_index(
    tera: web::Data<tera::Tera>
) -> Result<HttpResponse, Error> {
    // TODO qb与anime_task同步

    let broadcast_url = BroadcastUrl { url_year: 0, url_season : 0 };
    let broadcast_map = get_broadcast_map().await;
    let mut context = Context::new();
    context.insert("broadcast_map", &broadcast_map);
    context.insert("broadcast_url", &broadcast_url);
    context.insert("page_flag", &0);
    let rendered = tera.render("download.html", &context).expect("Failed to render template");
    Ok(HttpResponse::Ok().content_type("text/html").body(rendered))
}

#[get("/qb_download_progress")]
pub async fn qb_download_progress_handler(
    pool: web::Data<Pool>,
    qb: web::Data<QbitTaskExecutor>
) -> Result<HttpResponse, Error> {
    let db_connection = &mut pool.get().unwrap();
    Ok(
        match get_qb_download_progress(db_connection, qb)
            .await {
                Ok(data) => HttpResponse::Created().json(data),
                _ => HttpResponse::from(HttpResponse::InternalServerError()),
            },
    )
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskQbInfo {
    pub mikan_id: i32,
    pub anime_name: String,
    pub episode: i32,
    pub torrent_name: String,
    pub qb_info: TorrentInfo
}

pub async fn get_qb_download_progress(
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
    qb: web::Data<QbitTaskExecutor>
) -> Result<Vec<TaskQbInfo>, Error> {
    let mut task_qb_info_list: Vec<TaskQbInfo> = Vec::new();
    let task_list = dao::anime_task::get_by_qbtaskstatus(db_connection, 0).await.unwrap();
    let anime_map = get_anime_id_name_map(db_connection).await.unwrap();
    for t in task_list {
        let torrent_info = qb.qb_api_torrent_info(t.torrent_name.clone()).await.unwrap();
        task_qb_info_list.push(TaskQbInfo { 
            mikan_id     : t.mikan_id,
            anime_name   : anime_map.get(&t.mikan_id).unwrap().to_string(), 
            episode      : t.episode,
            torrent_name : t.torrent_name,
            qb_info      : torrent_info

        });
    }
    Ok(task_qb_info_list)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QbExecuteJson {
    pub torrent_name: String,
    pub execute_type: i32
}
// 1: delete
// 2: pause
// 3: resume

#[post("/qb_execute")]
pub async fn qb_execute_handler(
    item: web::Json<QbExecuteJson>,
    pool: web::Data<Pool>,
    qb: web::Data<QbitTaskExecutor>
) -> Result<HttpResponse, Error> {
    let db_connection = &mut pool.get().unwrap();
    Ok(
        match qb_execute(item, db_connection, qb)
            .await {
                Ok(data) => HttpResponse::Created().json(data),
                _ => HttpResponse::from(HttpResponse::InternalServerError()),
            },
    )
}

pub async fn qb_execute(
    item: web::Json<QbExecuteJson>,
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
    qb: web::Data<QbitTaskExecutor>
) -> Result<(), Error> {
    let torrent_name = item.torrent_name.clone();
    match item.execute_type {
        // delete
        1 => { 
            qb.qb_api_del_torrent(torrent_name.clone()).await.unwrap();
            dao::anime_task::delete_anime_task_by_torrent_name(db_connection, torrent_name).await.unwrap();
        },
        // pause
        2 => qb.qb_api_pause_torrent(torrent_name).await.unwrap(),
        // resume
        _ => qb.qb_api_resume_torrent(torrent_name).await.unwrap(),
    }
    Ok(())
}