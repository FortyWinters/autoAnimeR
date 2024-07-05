use actix_web::{web, App, HttpServer};
use api::do_anime_task;
use diesel::r2d2::{self, ConnectionManager};
use diesel::SqliteConnection;
use mods::{config::Config, qb_api::QbitTaskExecutor};
use routers::*;

mod api;
mod dao;
mod error;
mod models;
mod mods;
mod routers;
mod schema;
mod v2;
use actix::spawn;
use log4rs;
use std::sync::Arc;
use tokio::sync::RwLock as TokioRwLock;

#[macro_use]
extern crate diesel;

pub type Pool = r2d2::Pool<ConnectionManager<SqliteConnection>>;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    log4rs::init_file("./config/log4rs.yaml", Default::default()).unwrap();
    
    let config = Arc::new(TokioRwLock::new(Config::load_config("./config/config.yaml").await.unwrap()));
    
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let database_pool = Pool::builder()
        .build(ConnectionManager::<SqliteConnection>::new(database_url))
        .expect("Failed to create pool.");

    let conf = config.read().await;
    let qb = Arc::new(TokioRwLock::new(QbitTaskExecutor::new_with_config(&conf)
        .await
        .expect("Failed to create qb client")));
    drop(conf);

    let tastk_status = Arc::new(TokioRwLock::new(false));
    let video_file_lock = Arc::new(TokioRwLock::new(false));
    let mut db_connection = database_pool.get().unwrap();

    do_anime_task::add_default_filter(&config, &mut db_connection).await.unwrap();

    let qb_for_task = Arc::clone(&qb);
    let video_file_lock_for_task = Arc::clone(&video_file_lock);
    spawn(async move {
        let _ =
            do_anime_task::auto_update_rename_extract(&video_file_lock_for_task, &mut db_connection, &qb_for_task)
                .await;
    });

    let http_server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(database_pool.clone()))
            .app_data(web::Data::new(qb.clone()))
            .app_data(web::Data::new(tastk_status.clone()))
            .app_data(web::Data::new(video_file_lock.clone()))
            .app_data(web::Data::new(config.clone()))
            .configure(anime_routes_v2)
            .configure(setting_routes_v2)
            .configure(ws_routes_v2)
            .configure(video_routes_v2)
    })
    .bind(("0.0.0.0", 8080))?
    .run();

    http_server.await
}
