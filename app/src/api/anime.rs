use crate::{Pool, models::anime_list::AnimeListJson, mods::spider, dao};
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

    let anime_list_json: Vec<AnimeListJson> = anime_list.into_iter().map(|anime| {
        AnimeListJson {
            mikan_id:         anime.mikan_id,
            anime_name:       anime.anime_name,
            img_url:          anime.img_url,
            update_day:       anime.update_day,
            anime_type:       anime.anime_type,
            subscribe_status: anime.subscribe_status,
        }
    }).collect();

    Ok(dao::anime_list::add_vec(pool, anime_list_json).await.unwrap())
}

#[get("/")]
pub async fn index(tera: web::Data<tera::Tera>) -> Result<HttpResponse, Error> {
    let mut context = Context::new();
    context.insert("name", "无职转生");

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