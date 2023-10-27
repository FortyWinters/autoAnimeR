use actix_web::{post,get, web, HttpResponse, Error};
use anyhow::Result;
use tera::Context;
use crate::api::anime::{BroadcastUrl, get_broadcast_map};
use crate::api::do_anime_task;
use std::sync::Arc;
use tokio::sync::RwLock as TokioRwLock;
use crate::Pool;
use crate::mods::qb_api::QbitTaskExecutor;
use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize)]
pub struct TaskInterval {
    pub interval: i32
}

#[get("/")]
pub async fn setting_index_handler(
    tera: web::Data<tera::Tera>
) -> Result<HttpResponse, Error> {
    Ok(
        match setting_index(tera)
            .await {
                Ok(res) => res,
                _ => HttpResponse::from(HttpResponse::InternalServerError()),
            },
    )
}

pub async fn setting_index(
    tera: web::Data<tera::Tera>
) -> Result<HttpResponse, Error> {
    let broadcast_url = BroadcastUrl { url_year: 0, url_season : 0 };
    let broadcast_map = get_broadcast_map().await;
    let mut context = Context::new();
    context.insert("broadcast_map", &broadcast_map);
    context.insert("broadcast_url", &broadcast_url);
    context.insert("page_flag", &0);
    let rendered = tera.render("setting.html", &context).expect("Failed to render template");
    Ok(HttpResponse::Ok().content_type("text/html").body(rendered))
}

#[post("/exit")]
pub async fn exit_schedule_task_handler(
    status: web::Data<Arc<TokioRwLock<bool>>>
) -> Result<HttpResponse, Error> {
    do_anime_task::exit_task(&status).await;
    Ok(HttpResponse::Ok().body("ok"))
}

#[post("/start")]
pub async fn start_schedule_task_handler(
    status: web::Data<Arc<TokioRwLock<bool>>>,
    qb: web::Data<QbitTaskExecutor>,
    pool: web::Data<Pool>
) -> Result<HttpResponse, Error> {
    let run_handle = tokio::spawn(async move {
        do_anime_task::run_task(&status, &qb, &mut pool.get().unwrap()).await;
    });
    run_handle.await.unwrap();
    Ok(HttpResponse::Ok().body("ok"))
}

#[post("/change_interval")]
pub async fn change_task_interval_handler(
    item: web::Json<TaskInterval>,
    status: web::Data<Arc<TokioRwLock<bool>>>,
    qb: web::Data<QbitTaskExecutor>,
    pool: web::Data<Pool>
) -> Result<HttpResponse, Error> {
    do_anime_task::exit_task(&status).await;
    let run_handle = tokio::spawn(async move {
        do_anime_task::change_task_interval(item.interval, &status, &qb, &mut pool.get().unwrap()).await;
    });
    run_handle.await.unwrap();
    Ok(HttpResponse::Ok().body("ok"))
}