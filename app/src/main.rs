use actix_files::Files;
use actix_web::{web, App, HttpServer};
use api::do_anime_task;
use diesel::r2d2::{self, ConnectionManager};
use diesel::SqliteConnection;
use mods::qb_api::QbitTaskExecutor;
use routers::*;
use tera::Tera;

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

    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let database_pool = Pool::builder()
        .build(ConnectionManager::<SqliteConnection>::new(database_url))
        .expect("Failed to create pool.");

    let tera = Tera::new("templates/**/*.html").expect("Failed to load templates");

    // let mut tera: Tera = Tera::default();
    // match tera.add_raw_templates("templates/**/*") {
    //     Ok(_) => println!("ok"),
    //     Err(e) => panic!("error")
    // }

    let qb = QbitTaskExecutor::new_with_login("admin".to_string(), "adminadmin".to_string())
        .await
        .expect("Failed to create qb client");

    let tastk_status = Arc::new(TokioRwLock::new(false));
    let video_file_lock = Arc::new(TokioRwLock::new(false));

    let database_pool_for_server = database_pool.clone();
    let http_server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(database_pool_for_server.clone()))
            .app_data(web::Data::new(tera.clone()))
            .app_data(web::Data::new(qb.clone()))
            .app_data(web::Data::new(tastk_status.clone()))
            .service(Files::new("/static", "./static").show_files_listing())
            .configure(anime_routes)
            .configure(setting_routes)
            .configure(download_routes)
            .configure(anime_routes_v2)
            .configure(setting_routes_v2)
            .configure(ws_routes_v2)
    })
    .bind(("0.0.0.0", 8080))?
    .run();

    let database_pool_for_task = database_pool.clone();
    spawn(async move {
        let _ = do_anime_task::auto_update_and_rename(video_file_lock, database_pool_for_task).await;
    });

    http_server.await
}
