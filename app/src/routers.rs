use actix_web::web;
use crate::api::anime::*;

pub fn anime_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/anime")
            .service(my_anime_handler)
            .service(update_anime_list_handler)
            .service(anime_list_by_broadcast_handler)
            .service(subscribe_anime_handler)
            .service(cancel_subscribe_anime_handler)
            .service(update_anime_seed_handler)
            .service(my_anime_index_handler)
            .service(anime_detail_handler)
    );
}