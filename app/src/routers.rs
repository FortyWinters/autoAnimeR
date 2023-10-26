use actix_web::web;

pub fn anime_routes(cfg: &mut web::ServiceConfig) {
    use crate::api::anime::*;
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
            .service(create_task_by_seed_url_handler)
    );
}

pub fn setting_routes(cfg: &mut web::ServiceConfig) {
    use crate::api::setting::*;
    cfg.service(
        web::scope("/setting")
            .service(setting_index_handler)
            .service(exit_schedule_task_handler)
            .service(start_schedule_task_handler)
    );
}

pub fn download_routes(cfg: &mut web::ServiceConfig) {
    use crate::api::download::*;
    cfg.service(
        web::scope("/download")
            .service(download_index_handler)
            .service(qb_download_progress_handler)
            .service(qb_execute_handler)
    );
}