use actix_web::web;

pub fn anime_routes_v2(cfg: &mut web::ServiceConfig) {
    use crate::v2::anime::*;
    cfg.service(
        web::scope("/v2/anime")
            .service(get_anime_home_handler)
            .service(get_anime_info_handler)
            .service(subscribe_anime_handler)
            .service(get_anime_broadcast_handler)
            .service(update_anime_broadcast_handler)
            .service(get_anime_seed_handler)
            .service(get_subgroup_handler)
            .service(get_task_handler)
            .service(seed_update_handler)
            .service(seed_delete_handler)
            .service(seed_download_handler)
            .service(get_anime_detail_handler)
            .service(task_delete_handler)
            .service(task_update_handler)
    );
}

pub fn setting_routes_v2(cfg: &mut web::ServiceConfig) {
    use crate::v2::setting::*;
    cfg.service(
        web::scope("/v2/setting")
            .service(exit_schedule_task_handler)
            .service(start_schedule_task_handler)
            .service(change_task_interval_handler)
            .service(get_task_status_handler)
            .service(reload_task_handler)
            .service(relogin_qb_handler)
            .service(modify_config_handler)
    );
}

pub fn ws_routes_v2(cfg: &mut web::ServiceConfig) {
    use crate::v2::ws::*;
    cfg.service(web::resource("/v2/ws/").route(web::get().to(ws_index)));
}

pub fn video_routes_v2(cfg: &mut web::ServiceConfig) {
    use crate::v2::video::*;
    cfg.service(
        web::scope("/v2/video")
            .service(get_anime_task_handler)
            .service(get_subtitle_path_handler)
    );
}