use actix_web::{post, get, web, HttpResponse, Error};
use anyhow::Result;
use tera::Context;
use chrono::{Local, Datelike};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
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

// update anime list by year & season
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

    let db_connection = &mut pool.get().unwrap();
    dao::anime_list::add_vec(db_connection, anime_list_json_vec).await.unwrap();
    dao::anime_broadcast::add_vec(db_connection, anime_broadcast_json_vec).await.unwrap();

    // TODO 需多线程重构
    let  save_path = "static/img/anime_list".to_string();
    for img_url in &img_url_vec {
        if let Err(_) = mikan.download_img(img_url, &save_path).await {
            println!("download img failed, img_url:{}", img_url);
        }
    }

    Ok(anime_list.len())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BroadcastUrl {
    pub url_year: i32,
    pub url_season: i32
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
    let broadcast_url = BroadcastUrl { url_year, url_season };
    let anime_list = anime_list_by_broadcast(pool, url_year, url_season).await.unwrap();
    let broadcast_map = get_broadcast_map().await;
    let mut context = Context::new();
    context.insert("anime_list", &anime_list);
    context.insert("broadcast_map", &broadcast_map);
    context.insert("broadcast_url", &broadcast_url);
    context.insert("page_flag", &1);
    let rendered = tera.render("anime.html", &context).expect("Failed to render template");
    Ok(HttpResponse::Ok().content_type("text/html").body(rendered))
}

// show anime list by year & season
pub async fn anime_list_by_broadcast(
    pool: web::Data<Pool>,
    year: i32,
    season: i32
) -> Result<Vec<anime_list::AnimeList>, Error> {
    let db_connection = &mut pool.get().unwrap();
    let broadcast_list: Vec<anime_broadcast::AnimeBroadcast> = dao::anime_broadcast::get_by_year_season(db_connection, year, season).await.unwrap();
    let mut anime_list: Vec<anime_list::AnimeList> = Vec::new();
    for anime in &broadcast_list {
        anime_list.push(dao::anime_list::get_by_mikanid(db_connection, anime.mikan_id).await.unwrap());
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

// get year & season broadcast map
pub async fn get_broadcast_map() -> Vec<BroadcastMap> {
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

#[derive(Debug, Serialize, Deserialize)]
pub struct SubscribeAnimeJson {
    pub mikan_id: i32
}

#[post("/subscribe_anime")]
pub async fn subscribe_anime_handler(
    item: web::Json<SubscribeAnimeJson>,
    pool: web::Data<Pool>
) -> Result<HttpResponse, Error> {
    Ok(
        match subscribe_anime(item, pool)
            .await {
                Ok(mikan_id) => HttpResponse::Created().json(mikan_id),
                _ => HttpResponse::from(HttpResponse::InternalServerError()),
            },
    )
}

// subscribe anime by mikan id
pub async fn subscribe_anime(    
    item: web::Json<SubscribeAnimeJson>,
    pool: web::Data<Pool>
) -> Result<i32, Error> {
    let mikan_id = item.mikan_id;
    let db_connection = &mut pool.get().unwrap();
    if let Ok(_) = dao::anime_list::update_subscribestatus_by_mikanid(db_connection, mikan_id, 1).await {
        Ok(mikan_id)
    } else {
        Ok(-1)
    }
}

#[post("/cancel_subscribe_anime")]
pub async fn cancel_subscribe_anime_handler(
    item: web::Json<SubscribeAnimeJson>,
    pool: web::Data<Pool>
) -> Result<HttpResponse, Error> {
    Ok(
        match cancel_subscribe_anime(item, pool)
            .await {
                Ok(mikan_id) => HttpResponse::Created().json(mikan_id),
                _ => HttpResponse::from(HttpResponse::InternalServerError()),
            },
    )
}

// cancel subscribe anime by mikan id
pub async fn cancel_subscribe_anime(    
    item: web::Json<SubscribeAnimeJson>,
    pool: web::Data<Pool>
) -> Result<i32, Error> {
    let mikan_id = item.mikan_id;
    let db_connection = &mut pool.get().unwrap();
    if let Ok(_) = dao::anime_list::update_subscribestatus_by_mikanid(db_connection, mikan_id, 0).await {
        Ok(mikan_id)
    } else {
        Ok(-1)
    }
}

#[get("")]
pub async fn my_anime_handler(
    tera: web::Data<tera::Tera>,
    pool: web::Data<Pool>
) -> Result<HttpResponse, Error> {
    let broadcast_url = BroadcastUrl { url_year: 0, url_season : 0 };
    let anime_list = my_anime(pool).await.unwrap();
    let broadcast_map = get_broadcast_map().await;
    let mut context = Context::new();
    context.insert("anime_list", &anime_list);
    context.insert("broadcast_map", &broadcast_map);
    context.insert("broadcast_url", &broadcast_url);
    context.insert("page_flag", &0);
    let rendered = tera.render("anime.html", &context).expect("Failed to render template");
    Ok(HttpResponse::Ok().content_type("text/html").body(rendered))
}

pub async fn my_anime(
    pool: web::Data<Pool>
) -> Result<Vec<anime_list::AnimeList>, Error> {
    let db_connection = &mut pool.get().unwrap();
    let mut anime_vec = dao::anime_list::get_by_subscribestatus(db_connection, 1).await.unwrap();
    let task_vec = dao::anime_task::get_all(db_connection).await.unwrap();
    let mut task_mikan_id_set: HashSet<i32> = HashSet::new();
    for task in task_vec {
        if !task_mikan_id_set.contains(&task.mikan_id) {
            task_mikan_id_set.insert(task.mikan_id);
            if let Ok(anime) = dao::anime_list::get_by_mikanid(db_connection, task.mikan_id).await {
                if anime.subscribe_status == 0 {
                    task_mikan_id_set.insert(anime.mikan_id);
                    anime_vec.push(anime);
                }
            } else {
                println!("this anime is not in db, mikan_id: {}", task.mikan_id)
            }
        }
    }

    for anime in anime_vec.iter_mut() {
        let mut parts = anime.img_url.split('/');
        let img_name = parts.nth(4).unwrap();
        anime.img_url = format!("/static/img/anime_list/{}", img_name);
    }
    anime_vec.sort();
    Ok(anime_vec)
}   

// #[get("/get_all_anime_list")]
// pub async fn get_all_anime_list_handler(
//     pool: web::Data<Pool>
// ) -> Result<HttpResponse, Error> {
//     Ok(
//         match dao::anime_list::get_all(pool)
//             .await {
//                 Ok(anime_list) => HttpResponse::Created().json(anime_list),
//                 _ => HttpResponse::from(HttpResponse::InternalServerError()),
//             },
//     )
// }

// #[get("/get_all_anime_broadcast")]
// pub async fn get_all_anime_broadcast_handler(
//     pool: web::Data<Pool>
// ) -> Result<HttpResponse, Error> {
//     Ok(
//         match dao::anime_broadcast::get_all(pool)
//             .await {
//                 Ok(anime_broadcast) => HttpResponse::Created().json(anime_broadcast),
//                 _ => HttpResponse::from(HttpResponse::InternalServerError()),
//             },
//     )
// }