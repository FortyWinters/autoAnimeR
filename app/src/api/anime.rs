use actix_web::{post, get, web, HttpResponse, Error};
use anyhow::Result;
use tera::Context;
use chrono::{Local, Datelike};
use serde::{Deserialize, Serialize};
use crate::Pool;
use crate::dao;
use crate::mods::spider;
use crate::models::anime_list;
use crate::models::anime_broadcast;

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateAnimeListJson {
    pub year: i32,
    pub season: i32,
}

#[post("/update_anime_list")]
pub async fn update_anime_list_handler(
    pool: web::Data<Pool>,
    item: web::Json<UpdateAnimeListJson>
) -> Result<HttpResponse, Error> {
    Ok(
        match update_anime_list(item, pool).await {
            Ok(anime_list) => HttpResponse::Created().json(anime_list),
            _ => HttpResponse::from(HttpResponse::InternalServerError()),
        },
    )
}

pub async fn update_anime_list(
    item: web::Json<UpdateAnimeListJson>,
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
    let path_year = &path.0;
    let path_season = &path.1;
    let url_year: i32 = path_year.to_string().parse().unwrap();
    let url_season: i32 = path_season.to_string().parse().unwrap();

    let anime_list = anime_list_by_broadcast(pool, url_year, url_season).await.unwrap();
    let broadcast_map = get_broadcast_map();
    let mut context = Context::new();
    context.insert("anime_list", &anime_list);
    context.insert("broadcast_map", &broadcast_map);
    context.insert("url_year", &url_year);
    context.insert("url_season", &url_season);
    let rendered = tera.render("anime.html", &context).expect("Failed to render template");
    Ok(HttpResponse::Ok().content_type("text/html").body(rendered))
}

pub async fn anime_list_by_broadcast(
    pool: web::Data<Pool>,
    year: i32,
    season: i32
) -> Result<Vec<anime_list::AnimeList>, Error> {
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
    anime_list.sort();
    Ok(anime_list)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BroadcastMap {
    pub year: i32,
    pub spring: i32,
    pub summer:i32,
    pub autumn: i32,
    pub winter: i32,
}

pub fn get_broadcast_map() -> Vec<BroadcastMap> {
    let now = Local::now();
    let current_year = now.year();
    let current_month = now.month();
    let mut broadcast_map: Vec<BroadcastMap> = Vec::new();
    broadcast_map.push(BroadcastMap {
        year   : 2013, 
        spring : 0, 
        summer : 0, 
        autumn : 1, 
        winter : 0
    });

    let bm = BroadcastMap {
        year   : 1999, 
        spring : 1, 
        summer : 1, 
        autumn : 1, 
        winter : 1
    };
    for year in 2014..current_year {
        let mut b = bm.clone();
        b.year = year;
        broadcast_map.push(b);
    }

    let mut b = bm.clone();
    if current_month > 0 && current_month < 3 {
        b.year = current_year;
        b.spring = 0;
        b.summer = 0;
        b.autumn = 0;
    } else if current_month >= 3 && current_month < 6 {
        b.year = current_year;
        b.summer = 0;
        b.autumn = 0;
    } else if current_month >= 6 && current_month < 9 {
        b.year = current_year;
        b.autumn = 0;
    } else {
        b.year = current_year;
    }
    broadcast_map.push(b);
    return broadcast_map
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