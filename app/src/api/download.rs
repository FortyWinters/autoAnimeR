use actix_web::{get, web, HttpResponse, Error};
use anyhow::Result;
use tera::Context;
use crate::api::anime::{BroadcastUrl, get_broadcast_map};
use core::result::Result::Ok;
// use crate::mods::qb_api::{self, QbitTaskExecutor};

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

// #[get("/get_qb_download_progress")]
// pub async fn get_qb_download_progress_handler(
//     pool: web::Data<Pool>,
//     qb: web::Data<QbitTaskExecutor>
// ) -> Result<HttpResponse, Error> {
//     let db_connection = &mut pool.get().unwrap();
//     Ok(
//         match get_qb_download_progress(db_connection, qb)
//             .await {
//                 Ok(data) => HttpResponse::Created().json(data),
//                 _ => HttpResponse::from(HttpResponse::InternalServerError()),
//             },
//     )
// }

// #[derive(Debug, Serialize, Deserialize)]
// pub struct TaskQbInfo {
//     pub mikan_id: i32,
//     pub anime_name: String,
//     pub episode: i32,
//     pub torrent_name: String,
//     pub torrent_info: TorrentInfo
// }

// pub async fn get_qb_download_progress(
//     db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
//     qb: web::Data<QbitTaskExecutor>
// ) -> Result<Vec<TaskQbInfo>, Error> {
//     let mut qb_res: Vec<TaskQbInfo> = Vec::new();
//     let task_list = dao::anime_task::get_by_qbtaskstatus(db_connection, 0).await.unwrap();
//     if task_list.is_empty() {
//         Ok(qb_res)
//     }

//     let anime_list = dao::anime_list::get_all(db_connection).await.unwrap();
//     for a in anime_list {
//         let torrent_info = qb_api::
//     }

    




//     Ok(1)
// }