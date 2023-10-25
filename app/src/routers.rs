use actix_web::web;
use crate::api::anime::*;
use crate::api::setting::*;
use crate::api::download::*;

pub fn anime_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/anime")
            .service(anime_index_handler)
            .service(update_anime_list_handler)
            .service(anime_list_by_broadcast_handler)
            .service(subscribe_anime_handler)
            .service(cancel_subscribe_anime_handler)
            .service(update_anime_seed_handler)
            .service(anime_detail_handler)
            .service(recover_seed_handler)
            .service(delete_anime_data_handler)
    );
}

pub fn setting_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/setting")
            .service(setting_index_handler)
    );
}

pub fn download_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/download")
            .service(download_index_handler)
    );
}