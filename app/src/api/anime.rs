use actix_web::{post, get, web, HttpResponse, Error};
use anyhow::Result;
use tera::Context;
use chrono::{Local, Datelike};
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, HashMap};
use diesel::r2d2::{PooledConnection, ConnectionManager};
use diesel::SqliteConnection;
use crate::Pool;
use crate::dao;
use crate::mods::spider;
use crate::models::anime_list;
use crate::models::anime_broadcast;
use crate::models::anime_seed;
use crate::models::anime_subgroup;

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

#[get("list/{url_year}/{url_season}")]
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

#[get("/")]
pub async fn my_anime_index_handler(
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


#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateAnimeSeedJson {
    pub mikan_id: i32,
}

#[post("/update_anime_seed")]
pub async fn update_anime_seed_handler(
    item: web::Json<UpdateAnimeSeedJson>,
    pool: web::Data<Pool>
) -> Result<HttpResponse, Error> {
    let db_connection = &mut pool.get().unwrap();
    Ok(
        match get_anime_seed(item.mikan_id, db_connection)
            .await {
                Ok(seed_number) => HttpResponse::Created().json(seed_number),
                _ => HttpResponse::from(HttpResponse::InternalServerError()),
            },
    )
}

// TODO 单线程需重构
pub async fn get_anime_seed(    
    mikan_id: i32,
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
) -> Result<usize, Error> {
    let mikan = spider::Mikan::new()?;
    let anime_info = dao::anime_list::get_by_mikanid(db_connection, mikan_id).await.unwrap();
    let anime_type = anime_info.anime_type;
    let mut seed_vec: Vec<anime_seed::AnimeSeedJson> = Vec::new();
    let subgroup_list = mikan.get_subgroup(mikan_id).await.expect("get subgroup failed");
    for subgroup in &subgroup_list {
        if let Ok(seed_list) = get_seed_by_subgroup(mikan.clone(), mikan_id, subgroup.subgroup_id, anime_type).await {
            seed_vec.extend(seed_list);
        }
    }
    let number = seed_vec.len();
    dao::anime_seed::add_bulk(db_connection, seed_vec).await.unwrap();
    let anime_subgroup_list = convert_spider_subgroup_to_anime_subgroup(subgroup_list);
    dao::anime_subgroup::add_vec(db_connection, anime_subgroup_list).await.unwrap();
    Ok(number)
}

pub async fn get_seed_by_subgroup(
    mikan: spider::Mikan,
    mikan_id: i32,
    subgroup_id: i32,
    anime_type: i32
) -> Result<Vec<anime_seed::AnimeSeedJson>, Error> {
    let seed_list: Vec<spider::Seed> = mikan.get_seed(mikan_id, subgroup_id, anime_type).await.unwrap();
    Ok(convert_spider_seed_to_anime_seed(seed_list))
}

fn convert_spider_seed_to_anime_seed(spider_vec: Vec<spider::Seed>) -> Vec<anime_seed::AnimeSeedJson> {
    spider_vec.into_iter().map(|s| anime_seed::AnimeSeedJson {     
        mikan_id    : s.mikan_id,
        subgroup_id : s.subgroup_id,
        episode     : s.episode,
        seed_name   : s.seed_name,
        seed_url    : s.seed_url,
        seed_status : s.seed_status,
        seed_size   : s.seed_size 
    }).collect()
}

fn convert_spider_subgroup_to_anime_subgroup(spider_vec: Vec<spider::Subgroup>) -> Vec<anime_subgroup::AnimeSubgroupJson> {
    spider_vec.into_iter().map(|s| anime_subgroup::AnimeSubgroupJson {     
        subgroup_name : s.subgroup_name,
        subgroup_id   : s.subgroup_id,
    }).collect()
}

#[get("/detail/{mikan_id}")]
pub async fn anime_detail_handler(
    pool: web::Data<Pool>,
    tera: web::Data<tera::Tera>,
    path: web::Path<String>
) -> Result<HttpResponse, Error> {
    let db_connection = &mut pool.get().unwrap();
    let path_mikan_id = &path;
    let path_mikan_id: i32 = path_mikan_id.to_string().parse().unwrap();
    let broadcast_url = BroadcastUrl { url_year: 0, url_season : 0 };
    let broadcast_map = get_broadcast_map().await;
    let (anime, subgroup_with_seed_list) = get_anime_seed_group_by_subgroup(path_mikan_id, db_connection).await.unwrap();
    let mut context = Context::new();
    context.insert("anime", &anime);
    context.insert("subgroup_with_seed_list", &subgroup_with_seed_list);
    context.insert("broadcast_map", &broadcast_map);
    context.insert("broadcast_url", &broadcast_url);
    context.insert("page_flag", &0);
    let rendered = tera.render("detail.html", &context).expect("Failed to render template");
    Ok(HttpResponse::Ok().content_type("text/html").body(rendered))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubgroupWithSeed {
    pub subgroup_id: i32,
    pub subgroup_name: String,
    pub seed_list: Vec<SeedWithTask>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SeedWithTask {
    pub seed: anime_seed::AnimeSeed,
    pub status: i32
}
// 0: unused      grey
// 1: failed      black
// 2: downloading blue
// 3: downloaded  green

pub async fn get_anime_seed_group_by_subgroup(
    mikan_id: i32,
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
) -> Result<(anime_list::AnimeList, Vec<SubgroupWithSeed>), Error> {
    let mut anime = dao::anime_list::get_by_mikanid(db_connection, mikan_id).await.unwrap();
    let mut parts = anime.img_url.split('/');
    let img_name = parts.nth(4).unwrap();
    anime.img_url = format!("/static/img/anime_list/{}", img_name);

    let task_list = dao::anime_task::get_exist_anime_task_by_mikan_id(db_connection, mikan_id).await.unwrap();
    let mut task_episode_map: HashMap<i32, i32> = HashMap::new();
    let mut task_url_map: HashMap<String, i32> = HashMap::new();
    for task in task_list {
        task_episode_map.insert(task.episode, task.qb_task_status);
        task_url_map.insert(task.torrent_name, task.qb_task_status);
    }   
    
    let mut subgroup_with_seed_list: Vec<SubgroupWithSeed> = Vec::new();
    let seed_list = dao::anime_seed::get_anime_seed_by_mikan_id(db_connection, mikan_id).await.unwrap();
    let mut seed_episode_set: HashSet<i32> = HashSet::new();
    for seed in seed_list {
        seed_episode_set.insert(seed.episode);
    }
    if seed_episode_set.is_empty() {
        return Ok((anime, subgroup_with_seed_list));
    }

    let mut seed_list_0: Vec<SeedWithTask> = Vec::new();
    for epi in seed_episode_set {
        let seed = anime_seed::AnimeSeed {
            id          : Some(0),
            mikan_id,
            subgroup_id : 0,
            episode     : epi,
            seed_name   : "null".to_string(),
            seed_url    : "null".to_string(),
            seed_status : 0,
            seed_size   : "null".to_string()
        };

        let epi_status: i32;
        if let Some(status) = task_episode_map.get(&epi) {
            epi_status = *status + 2;
        } else {
            epi_status = 0;
        }

        seed_list_0.push(SeedWithTask { 
            seed, 
            status: epi_status   
        });
    }
    seed_list_0.sort_by_key(|s| s.seed.episode);
    subgroup_with_seed_list.push(SubgroupWithSeed {
        subgroup_id: 0,
        subgroup_name: "更新集数".to_string(),
        seed_list: seed_list_0
    });

    let subgroup_list = dao::anime_subgroup::get_all(db_connection).await.unwrap();
    for subgroup in subgroup_list {
        let seed_list = dao::anime_seed::get_by_mikanid_subgeoupid(db_connection, mikan_id, subgroup.subgroup_id).await.unwrap();
        if seed_list.is_empty() {
            continue;
        }
        let mut seed_with_task_list: Vec<SeedWithTask> = Vec::new();
        for seed in seed_list {
            let status: i32;
            if seed.seed_status == 0 {
                status = 0;
            } else {
                if let Some(task_status) = task_url_map.get(&seed.seed_url) {
                    status = *task_status + 2;
                } else {
                    status = 1;
                }
            }
            seed_with_task_list.push(SeedWithTask { 
                seed, 
                status
            });
        }
        seed_with_task_list.sort_by_key(|a| a.seed.episode);
        subgroup_with_seed_list.push(SubgroupWithSeed { 
            subgroup_id   : subgroup.subgroup_id, 
            subgroup_name : subgroup.subgroup_name, 
            seed_list     : seed_with_task_list
        })
    }
    subgroup_with_seed_list.sort_by_key(|a| a.subgroup_id);
    
    Ok((anime, subgroup_with_seed_list))
}

