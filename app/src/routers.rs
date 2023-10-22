// use actix_files::Files;
use actix_web::web;
use crate::api::anime::*;



// pub fn static_routes(cfg: &mut web::ServiceConfig) {
//     cfg.service(Files::new("/static", "./static").show_files_listing());
// }

pub fn anime_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/anime")
            .service(add_anime_list)
            .service(get_all_anime_list)
            .service(del_anime_list)
            .service(index)
    );
}