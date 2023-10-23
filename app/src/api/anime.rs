use actix_web::{post, get, web, HttpResponse, Error};
use anyhow::Result;
use tera::Context;
use crate::Pool;
use crate::dao;
use crate::mods::spider;
use crate::models::anime_list;
use crate::models::anime_broadcast;

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
) -> Result<usize, Error> {
    let mikan = spider::Mikan::new()?;
    let anime_list = mikan.get_anime(item.year, item.season).await?;
    let mut anime_list_json_vec: Vec<anime_list::AnimeListJson> = Vec::new();
    let mut anime_broadcast_json_vec: Vec<anime_broadcast::AnimeBroadcastJson> = Vec::new();
    let mut img_url_vec: Vec<String> = Vec::new();

    for anime in &anime_list {
        anime_list_json_vec.push(anime_list::AnimeListJson {
            mikan_id         : anime.mikan_id,
            anime_name       : anime.anime_name.clone(),
            img_url          : anime.img_url.clone(),
            update_day       : anime.update_day,
            anime_type       : anime.anime_type,
            subscribe_status : anime.subscribe_status,
        });
        anime_broadcast_json_vec.push(anime_broadcast::AnimeBroadcastJson {
            mikan_id : anime.mikan_id,
            year     : item.year,
            season   : item.season
        });
        img_url_vec.push(anime.img_url.clone());
    }

    dao::anime_list::add_vec(pool.clone(), anime_list_json_vec).await.unwrap();
    dao::anime_broadcast::add_vec(pool.clone(), anime_broadcast_json_vec).await.unwrap();

    // TODO 需多线程重构
    let  save_path = "static/img/anime_list".to_string();
    for img_url in &img_url_vec {
        if let Err(_) = mikan.download_img(img_url, &save_path).await {
            println!("download img failed, img_url:{}", img_url);
        }
    }

    Ok(anime_list.len())
}

#[get("/{url_year}/{url_season}")]
pub async fn anime_list_by_broadcast_handler(
    pool: web::Data<Pool>,
    tera: web::Data<tera::Tera>,
    path: web::Path<(String, String)>
) -> Result<HttpResponse, Error> {
    let anime_list = anime_list_by_broadcast(pool, path).await.unwrap();
    let mut context = Context::new();
    context.insert("anime_list", &anime_list);
    let rendered = tera.render("anime.html", &context).expect("Failed to render template");
    Ok(HttpResponse::Ok().content_type("text/html").body(rendered))
}

pub async fn anime_list_by_broadcast(
    pool: web::Data<Pool>,
    path: web::Path<(String, String)>
) -> Result<Vec<anime_list::AnimeList>, Error> {
    let path_year = &path.0;
    let path_season = &path.1;
    let year: i32 = path_year.to_string().parse().unwrap();
    let season: i32 = path_season.to_string().parse().unwrap();
    let broadcast_list: Vec<anime_broadcast::AnimeBroadcast> = dao::anime_broadcast::get_by_year_season(pool.clone(), year, season).await.unwrap();
    let mut anime_list: Vec<anime_list::AnimeList> = Vec::new();
    for anime in &broadcast_list {
        anime_list.push(dao::anime_list::get_by_mikanid(pool.clone(), anime.mikan_id).await.unwrap());
    }

    for anime in anime_list.iter_mut() {
        let mut parts = anime.img_url.split('/');
        let img_name = parts.nth(4).unwrap();
        anime.img_url = format!("/static/img/anime_list/{}", img_name);
    }
    // TODO 番剧排序
    Ok(anime_list)
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