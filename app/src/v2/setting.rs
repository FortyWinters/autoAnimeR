use crate::api::do_anime_task;
use crate::mods::qb_api::QbitTaskExecutor;
use crate::Pool;
use actix_web::{get, post, web, Error, HttpResponse};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock as TokioRwLock;
#[derive(Debug, Serialize, Deserialize)]
pub struct TaskInterval {
    pub interval: i32,
}

#[post("/exit")]
pub async fn exit_schedule_task_handler(
    status: web::Data<Arc<TokioRwLock<bool>>>,
) -> Result<HttpResponse, Error> {
    do_anime_task::exit_task(&status).await;
    Ok(HttpResponse::Ok().body("ok"))
}

#[post("/start")]
pub async fn start_schedule_task_handler(
    status: web::Data<Arc<TokioRwLock<bool>>>,
    qb: web::Data<Arc<TokioRwLock<QbitTaskExecutor>>>,
    pool: web::Data<Pool>,
) -> Result<HttpResponse, Error> {
    let qb_clone = Arc::clone(&qb);
    let run_handle = tokio::spawn(async move {
        let qb_read = qb_clone.read().await;
        do_anime_task::run_task(&status, &qb_read, &mut pool.get().unwrap()).await;
    });
    drop(run_handle);
    Ok(HttpResponse::Ok().body("ok"))
}

#[post("/change_interval")]
pub async fn change_task_interval_handler(
    item: web::Json<TaskInterval>,
    status: web::Data<Arc<TokioRwLock<bool>>>,
    qb: web::Data<Arc<TokioRwLock<QbitTaskExecutor>>>,
    pool: web::Data<Pool>,
) -> Result<HttpResponse, Error> {
    do_anime_task::exit_task(&status).await;
    let qb_clone = Arc::clone(&qb);
    let run_handle = tokio::spawn(async move {
        let qb_read = qb_clone.read().await;
        do_anime_task::change_task_interval(item.interval, &status, &qb_read, &mut pool.get().unwrap())
            .await;
    });
    drop(run_handle);
    Ok(HttpResponse::Ok().body("ok"))
}

#[get("/get_task_status")]
pub async fn get_task_status_handler(
    status: web::Data<Arc<TokioRwLock<bool>>>,
) -> Result<HttpResponse, Error> {
    Ok(match do_anime_task::get_task_status(&status).await {
        Ok(task_status) => {
            if task_status {
                HttpResponse::Ok().body("Task is Running")
            } else {
                HttpResponse::Ok().body("Task is not Running")
            }
        }
        Err(_) => HttpResponse::from(HttpResponse::InternalServerError()),
    })
}

#[get("/reload_task")]
pub async fn reload_task_handler(
    video_file_lock: web::Data<Arc<TokioRwLock<bool>>>,
    pool: web::Data<Pool>,
    qb: web::Data<Arc<TokioRwLock<QbitTaskExecutor>>>,
) -> Result<HttpResponse, Error> {
    let db_connection = &mut pool.get().unwrap();
    let qb = qb.read().await;
    Ok(
        match do_anime_task::create_anime_task_from_exist_files(
            &video_file_lock,
            db_connection,
            &qb,
        )
        .await
        {
            Ok(_) => HttpResponse::Ok().body("ok"),
            Err(_) => HttpResponse::from(HttpResponse::InternalServerError()),
        },
    )
}

#[get("/relogin_qb")]
pub async fn relogin_qb_handler(
    qb: web::Data<Arc<TokioRwLock<QbitTaskExecutor>>>,
) -> Result<HttpResponse, Error> {
    let mut qb = qb.write().await;
    Ok(match qb.relogin().await {
        Ok(_) => HttpResponse::Ok().body("ok"),
        Err(_) => HttpResponse::from(HttpResponse::InternalServerError()),
    })
}
