use crate::{Pool, models::anime_list::AnimeListJson, models::anime_broadcast::AnimeBroadcastJson, mods::spider, dao};
use actix_web::{post, get, web, HttpResponse, Error};
use anyhow::Result;
use tera::Context;

#[post("/update_anime_list")]
pub async fn update_anime_list_handler(
    pool: web::Data<Pool>,
    item: web::Json<spider::UpdateAnimeListJson>
) -> Result<HttpResponse, Error> {
    Ok(
        match update_anime_list(item, pool).await {
            Ok(anime_list) => HttpResponse::Created().json(anime_list),
            _ => HttpResponse::from(HttpResponse::InternalServerError()),
        },
    )
}

pub async fn update_anime_list(
    item: web::Json<spider::UpdateAnimeListJson>,
    pool: web::Data<Pool>
) -> Result<i32, Error> {
    let mikan = spider::Mikan::new()?;
    let anime_list = mikan.get_anime(item.year, item.season).await?;
    let mut anime_list_json_vec: Vec<AnimeListJson> = Vec::new();
    let mut anime_broadcast_json_vec: Vec<AnimeBroadcastJson> = Vec::new();

    for anime in &anime_list {
        anime_list_json_vec.push(AnimeListJson {
            mikan_id         : anime.mikan_id,
            anime_name       : anime.anime_name.clone(),
            img_url          : anime.img_url.clone(),
            update_day       : anime.update_day,
            anime_type       : anime.anime_type,
            subscribe_status : anime.subscribe_status,
        });
        anime_broadcast_json_vec.push(AnimeBroadcastJson {
            mikan_id : anime.mikan_id,
            year     : item.year,
            season   : item.season
        });
    }
    dao::anime_list::add_vec(pool.clone(), anime_list_json_vec).await.unwrap();
    dao::anime_broadcast::add_vec(pool.clone(), anime_broadcast_json_vec).await.unwrap();
    Ok(1)
}

#[get("/")]
pub async fn index(
    tera: web::Data<tera::Tera>,
    pool: web::Data<Pool>
) -> Result<HttpResponse, Error> {
    let anime_list = dao::anime_list::get_all(pool).await.unwrap();
    let mut context = Context::new();
    context.insert("anime_list", &anime_list);
    let rendered = tera.render("index.html", &context).expect("Failed to render template");
    Ok(HttpResponse::Ok().content_type("text/html").body(rendered))
}

#[get("/get_all_anime_list")]
pub async fn get_all_anime_list_handler(
    pool: web::Data<Pool>
) -> Result<HttpResponse, Error> {
    Ok(
        match dao::anime_list::get_all(pool)
            .await {
                Ok(anime_list) => HttpResponse::Created().json(anime_list),
                _ => HttpResponse::from(HttpResponse::InternalServerError()),
            },
    )
}

#[get("/get_all_anime_broadcast")]
pub async fn get_all_anime_broadcast_handler(
    pool: web::Data<Pool>
) -> Result<HttpResponse, Error> {
    Ok(
        match dao::anime_broadcast::get_all(pool)
            .await {
                Ok(anime_broadcast) => HttpResponse::Created().json(anime_broadcast),
                _ => HttpResponse::from(HttpResponse::InternalServerError()),
            },
    )
}