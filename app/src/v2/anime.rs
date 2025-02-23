use crate::api::do_anime_task;
use crate::dao;
use crate::models::anime_subgroup::AnimeSubgroup;
use crate::models::{anime_broadcast, anime_list, anime_seed, anime_subgroup, anime_task};
use crate::mods::spider::BangumiInfo;
use crate::mods::spider::{self, Mikan};
use crate::register_handler;
use crate::v2::common::handle_error;
use crate::{WebData, DB};
use actix_web::{web, Error, HttpResponse};
use anyhow::Result;
use futures::future::join_all;
use log;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Serialize, Deserialize)]
pub struct AnimeMikanIdReqJson {
    pub mikan_id: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnimeSubscribeReqJson {
    pub mikan_id: i32,
    pub subscribe_status: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnimeBroadcastReqJson {
    pub year: i32,
    pub season: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnimeKeyWordReqJson {
    key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SeedReqJson {
    pub mikan_id: i32,
    pub subgroup_id: i32,
    pub episode: i32,
    pub seed_name: String,
    pub seed_url: String,
    pub seed_status: i32,
    pub seed_size: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnimeDetail {
    pub anime_info: anime_list::AnimeList,
    pub seed_info: Vec<anime_seed::AnimeSeed>,
    pub subgroup_info: Vec<anime_subgroup::AnimeSubgroup>,
    pub task_info: Vec<anime_task::AnimeTask>,
}

register_handler!(GET "/home" => get_anime_home);
register_handler!(POST "/broadcast" => get_anime_broadcast, AnimeBroadcastReqJson);
register_handler!(POST "/broadcast/update" => update_anime_broadcast, AnimeBroadcastReqJson);
register_handler!(POST "/subscribe" => subscribe_anime, AnimeSubscribeReqJson);
register_handler!(POST "/search" => search_anime, AnimeKeyWordReqJson);
register_handler!(GET "/subgroup" => get_subgroup);
register_handler!(POST "/seed" => get_anime_seed, AnimeMikanIdReqJson);
register_handler!(POST "/seed/update" => seed_update, AnimeMikanIdReqJson);
register_handler!(POST "/seed/download" => seed_download, SeedReqJson);
register_handler!(POST "seed/delete" => seed_delete, AnimeMikanIdReqJson);
register_handler!(POST "/task" => get_task, AnimeMikanIdReqJson);
register_handler!(GET "/task/update" => task_update);
register_handler!(POST "/task/delete" => task_delete, SeedReqJson);
register_handler!(POST "/detail" => get_anime_detail, AnimeMikanIdReqJson);

async fn get_anime_home(web_data: web::Data<WebData>) -> Result<Vec<anime_list::AnimeList>, Error> {
    let db = &mut web_data
        .pool
        .get()
        .map_err(|e| handle_error(e, "failed to get db connection"))?;

    let mut anime_vec = dao::anime_list::get_by_subscribestatus(db, 1)
        .await
        .map_err(|e| {
            handle_error(
                e,
                "get_anime_home, dao::anime_list::get_by_subscribestatus failed",
            )
        })?;

    let task_vec = dao::anime_task::get_all(db)
        .await
        .map_err(|e| handle_error(e, "get_anime_home, dao::anime_task::get_all failed"))?;

    let mut task_mikan_id_set: HashSet<i32> = HashSet::new();
    for task in task_vec {
        if !task_mikan_id_set.insert(task.mikan_id) {
            continue;
        }
        if let Ok(anime) = dao::anime_list::get_by_mikanid(db, task.mikan_id)
            .await
            .map_err(|e| handle_error(e, "get_anime_home, dao::anime_list::get_by_mikanid failed"))
        {
            if anime.subscribe_status == 0 {
                anime_vec.push(anime);
            }
        }
    }

    for anime in anime_vec.iter_mut() {
        anime.img_url =
            get_img_name_from_url(&anime.img_url).unwrap_or_else(|| anime.img_url.clone());
    }
    anime_vec.sort();
    Ok(anime_vec)
}

async fn subscribe_anime(
    web_data: web::Data<WebData>,
    item: web::Json<AnimeSubscribeReqJson>,
) -> Result<(), Error> {
    let db = &mut web_data
        .pool
        .get()
        .map_err(|e| handle_error(e, "failed to get db connection"))?;

    let mikan_id = item.mikan_id;
    let subscribe_status = if item.subscribe_status == 1 { 0 } else { 1 };

    dao::anime_list::update_subscribestatus_by_mikanid(db, mikan_id, subscribe_status)
        .await
        .map_err(|e| {
            handle_error(
                e,
                "subscribe_anime, dao::anime_list::update_subscribestatus_by_mikanid failed",
            )
        })
}

async fn get_anime_detail(
    web_data: web::Data<WebData>,
    item: web::Json<AnimeMikanIdReqJson>,
) -> Result<AnimeDetail, Error> {
    let db = &mut web_data
        .pool
        .get()
        .map_err(|e| handle_error(e, "failed to get db connection"))?;

    let mikan_id = item.mikan_id;
    let tmp_item = web::Json(AnimeMikanIdReqJson { mikan_id });

    task_update(web_data.clone())
        .await
        .map_err(|e| handle_error(e, "get_anime_detail, task_update failed"))?;
    let anime = get_anime_info(db, mikan_id)
        .await
        .map_err(|e| handle_error(e, "get_anime_detail, get_anime_info failed"))?;
    let seed_vec = get_anime_seed(web_data.clone(), tmp_item)
        .await
        .map_err(|e| handle_error(e, "get_anime_detail, get_anime_seed failed"))?;
    let task_vec = get_task(web_data.clone(), item)
        .await
        .map_err(|e| handle_error(e, "get_anime_detail, get_task failed"))?;
    let subgroup_vec = get_subgroup(web_data.clone())
        .await
        .map_err(|e| handle_error(e, "get_anime_detail, get_subgroup failed"))?;
    let reorderd_subgroups = reoder_subgroups(subgroup_vec);

    let anime_detail = AnimeDetail {
        anime_info: anime,
        seed_info: seed_vec,
        subgroup_info: reorderd_subgroups,
        task_info: task_vec,
    };
    Ok(anime_detail)
}

async fn get_anime_info(db: &mut DB, mikan_id: i32) -> Result<anime_list::AnimeList, Error> {
    let mut anime_info = dao::anime_list::get_by_mikanid(db, mikan_id)
        .await
        .map_err(|e| handle_error(e, "get_anime_info, dao::anime_list::get_by_mikanid failed"))?;

    anime_info.img_url =
        get_img_name_from_url(&anime_info.img_url).unwrap_or_else(|| anime_info.img_url.clone());
    Ok(anime_info)
}

async fn get_anime_broadcast(
    web_data: web::Data<WebData>,
    item: web::Json<AnimeBroadcastReqJson>,
) -> Result<Vec<anime_list::AnimeList>, Error> {
    let db = &mut web_data
        .pool
        .get()
        .map_err(|e| handle_error(e, "failed to get db connection"))?;

    let broadcast_list: Vec<anime_broadcast::AnimeBroadcast> =
        dao::anime_broadcast::get_by_year_season(db, item.year, item.season)
            .await
            .map_err(|e| {
                handle_error(
                    e,
                    "get_anime_broadcast, dao::anime_broadcast::get_by_year_season failed",
                )
            })?;

    let mut anime_vec: Vec<anime_list::AnimeList> = Vec::new();

    for anime_broadcast in &broadcast_list {
        let mut anime = dao::anime_list::get_by_mikanid(db, anime_broadcast.mikan_id)
            .await
            .map_err(|e| {
                handle_error(
                    e,
                    "get_anime_broadcast, dao::anime_list::get_by_mikanid failed",
                )
            })?;

        anime.img_url =
            get_img_name_from_url(&anime.img_url).unwrap_or_else(|| anime.img_url.clone());

        anime_vec.push(anime);
    }
    anime_vec.sort();
    Ok(anime_vec)
}

async fn update_anime_broadcast(
    web_data: web::Data<WebData>,
    item: web::Json<AnimeBroadcastReqJson>,
) -> Result<(), Error> {
    let db = &mut web_data
        .pool
        .get()
        .map_err(|e| handle_error(e, "failed to get db connection"))?;

    let year = item.year;
    let season = item.season;

    let mikan = spider::Mikan::new()?;
    let anime_list = mikan.get_anime(year, season).await?;
    let mut anime_list_json_vec: Vec<anime_list::AnimeListJson> = Vec::new();
    let mut anime_broadcast_json_vec: Vec<anime_broadcast::AnimeBroadcastJson> = Vec::new();
    let mut img_url_vec: Vec<String> = Vec::new();

    for anime in &anime_list {
        anime_list_json_vec.push(anime_list::AnimeListJson {
            mikan_id: anime.mikan_id,
            anime_name: anime.anime_name.clone(),
            img_url: anime.img_url.clone(),
            update_day: anime.update_day,
            anime_type: anime.anime_type,
            subscribe_status: anime.subscribe_status,
            bangumi_id: -1,
            bangumi_rank: "".to_string(),
            bangumi_summary: "".to_string(),
            website: "".to_string(),
            anime_status: -1,
            total_episodes: -1,
            new_finished_episode: 0,
        });
        anime_broadcast_json_vec.push(anime_broadcast::AnimeBroadcastJson {
            mikan_id: anime.mikan_id,
            year: item.year,
            season: item.season,
        });
        img_url_vec.push(anime.img_url.clone());
    }

    dao::anime_list::add_vec(db, anime_list_json_vec)
        .await
        .map_err(|e| handle_error(e, "update_anime_broadcast, dao::anime_list::add_vec failed"))?;

    dao::anime_broadcast::add_vec(db, anime_broadcast_json_vec)
        .await
        .map_err(|e| {
            handle_error(
                e,
                "update_anime_broadcast, dao::anime_broadcast::add_vec failed",
            )
        })?;

    let config = web_data.config.read().await;
    let save_path = config.img_path.clone();
    drop(config);

    if !img_url_vec.is_empty() {
        let _ = join_all(
            img_url_vec
                .into_iter()
                .map(|img_url| download_anime_img(img_url, &save_path, &mikan)),
        )
        .await;
    }

    Ok(())
}

pub async fn download_anime_img(
    img_url: String,
    save_path: &str,
    mikan: &Mikan,
) -> Result<(), Error> {
    Ok(mikan.download_img(&img_url, save_path).await?)
}

async fn get_anime_seed(
    web_data: web::Data<WebData>,
    item: web::Json<AnimeMikanIdReqJson>,
) -> Result<Vec<anime_seed::AnimeSeed>, Error> {
    let db = &mut web_data
        .pool
        .get()
        .map_err(|e| handle_error(e, "failed to get db connection"))?;

    let seed_info = dao::anime_seed::get_anime_seed_by_mikan_id(db, item.mikan_id)
        .await
        .map_err(|e| {
            handle_error(
                e,
                "get_anime_seed, dao::anime_seed::get_anime_seed_by_mikan_id failed",
            )
        })?;

    Ok(seed_info)
}

async fn get_subgroup(
    web_data: web::Data<WebData>,
) -> Result<Vec<anime_subgroup::AnimeSubgroup>, Error> {
    let db = &mut web_data
        .pool
        .get()
        .map_err(|e| handle_error(e, "failed to get db connection"))?;

    let subgroup_info = dao::anime_subgroup::get_all(db)
        .await
        .map_err(|e| handle_error(e, "get_subgroup, dao::anime_subgroup::get_all failed"))?;

    Ok(subgroup_info)
}

async fn get_task(
    web_data: web::Data<WebData>,
    item: web::Json<AnimeMikanIdReqJson>,
) -> Result<Vec<anime_task::AnimeTask>, Error> {
    let db = &mut web_data
        .pool
        .get()
        .map_err(|e| handle_error(e, "failed to get db connection"))?;

    let task_info = dao::anime_task::get_exist_anime_task_by_mikan_id(db, item.mikan_id)
        .await
        .map_err(|e| {
            handle_error(
                e,
                "get_task, dao::anime_task::get_exist_anime_task_by_mikan_id failed",
            )
        })?;

    Ok(task_info)
}

pub async fn seed_update(
    web_data: web::Data<WebData>,
    item: web::Json<AnimeMikanIdReqJson>,
) -> Result<(), Error> {
    let db = &mut web_data
        .pool
        .get()
        .map_err(|e| handle_error(e, "failed to get db connection"))?;

    let mikan = spider::Mikan::new()?;
    let bangumi = spider::Bangumi::new()?;

    let mikan_id = item.mikan_id;
    let (bangumi_id, total_episodes) = mikan.get_bangumi_id_and_total_episodes(mikan_id).await?;

    let mut bangumi_info = BangumiInfo {
        bangumi_id,
        total_episodes,
        bangumi_rank: "暂无".to_string(),
        bangumi_summary: "暂无".to_string(),
        website: "暂无".to_string(),
    };

    match bangumi.get_bangumi_info(bangumi_id).await {
        Ok(info) => {
            bangumi_info = info;
        }
        Err(e) => {
            log::error!("get bangumi info failed, just bypass: {:?}", e);
        }
    }

    if bangumi_info.total_episodes == -1 {
        bangumi_info.total_episodes = total_episodes;
    }

    dao::anime_list::update_bangumiinfo_by_mikanid(
        db,
        mikan_id,
        anime_list::BangumiInfoJson {
            bangumi_id: bangumi_info.bangumi_id,
            bangumi_rank: bangumi_info.bangumi_rank,
            bangumi_summary: bangumi_info.bangumi_summary,
            website: bangumi_info.website,
            total_episodes: bangumi_info.total_episodes,
        },
    )
    .await
    .map_err(|e| {
        handle_error(
            e,
            "update_seed, dao::anime_list::update_bangumiinfo_by_mikanid failed",
        )
    })?;

    let anime_info = dao::anime_list::get_by_mikanid(db, mikan_id)
        .await
        .map_err(|e| handle_error(e, "seed_update, dao::anime_list::get_by_mikanid failed"))?;

    let anime_type = anime_info.anime_type;

    let subgroup_list = mikan.get_subgroup(mikan_id).await?;

    let mut subgroup_id_vec: Vec<i32> = Vec::new();
    for s in &subgroup_list {
        subgroup_id_vec.push(s.subgroup_id);
    }

    let anime_subgroup_list = convert_spider_subgroup_to_anime_subgroup(subgroup_list);

    dao::anime_subgroup::add_vec(db, anime_subgroup_list)
        .await
        .map_err(|e| handle_error(e, "update_seed, dao::anime_subgroup::add_vec failed"))?;

    let mut seed_vec: Vec<anime_seed::AnimeSeedJson> = Vec::new();
    if !subgroup_id_vec.is_empty() {
        let task_res_vec = join_all(subgroup_id_vec.into_iter().map(|subgroup_id| {
            get_anime_seed_by_spider(mikan_id, subgroup_id, anime_type, &mikan)
        }))
        .await;

        for task_res in task_res_vec {
            match task_res {
                Ok(seed_list) => {
                    seed_vec.extend(seed_list);
                }
                Err(_) => continue,
            }
        }
    }

    let max_episode = seed_vec.iter().map(|seed| seed.episode).max().unwrap_or(-1);
    if max_episode == bangumi_info.total_episodes {
        dao::anime_list::update_animestatus_by_mikanid(db, mikan_id, 1)
            .await
            .map_err(|e| {
                handle_error(
                    e,
                    "seed_update, dao::anime_list::update_animestatus_by_mikanid failed",
                )
            })?;
    } else {
        dao::anime_list::update_animestatus_by_mikanid(db, mikan_id, 0)
            .await
            .map_err(|e| {
                handle_error(
                    e,
                    "seed_update, dao::anime_list::update_animestatus_by_mikanid failed",
                )
            })?;
    }

    dao::anime_seed::add_bulk(db, seed_vec)
        .await
        .map_err(|e| handle_error(e, "update_seed, dao::anime_seed::add_bulk failed"))?;

    Ok(())
}

pub async fn seed_delete(
    web_data: web::Data<WebData>,
    item: web::Json<AnimeMikanIdReqJson>,
) -> Result<(), Error> {
    let db = &mut web_data
        .pool
        .get()
        .map_err(|e| handle_error(e, "failed to get db connection"))?;

    dao::anime_seed::delete_anime_seed_by_mikan_id(db, item.mikan_id)
        .await
        .map_err(|e| {
            handle_error(
                e,
                "seed_delete, dao::anime_seed::delete_anime_seed_by_mikan_id",
            )
        })?;

    dao::anime_task::delete_anime_task_by_mikan_id(db, item.mikan_id)
        .await
        .map_err(|e| {
            handle_error(
                e,
                "seed_delete, dao::anime_task::delete_anime_task_by_mikan_id",
            )
        })?;

    Ok(())
}

pub async fn seed_download(
    web_data: web::Data<WebData>,
    item: web::Json<SeedReqJson>,
) -> Result<(), Error> {
    let db = &mut web_data
        .pool
        .get()
        .map_err(|e| handle_error(e, "failed to get db connection"))?;

    let mikan = spider::Mikan::new()?;
    let qb = web_data.qb.read().await;

    if !qb.is_login {
        return Err(handle_error(
            anyhow::Error::msg("qbittorrent client not started"),
            "failed to down load seed",
        ));
    }

    dao::anime_seed::update_seedstatus_by_seedurl(db, &item.seed_url, 1)
        .await
        .map_err(|e| {
            handle_error(
                e,
                "seed_download, dao::anime_seed::update_seedstatus_by_seedurl failed",
            )
        })?;

    let anime_seed = convert_json_seed_to_anime_seed(item);

    do_anime_task::create_anime_task_by_seed(&mikan, anime_seed, &qb, db)
        .await
        .map_err(|e| {
            handle_error(
                e,
                "seed_download do_anime_task::create_anime_task_by_seed failed",
            )
        })?;
    Ok(())
}

pub async fn task_delete(
    web_data: web::Data<WebData>,
    item: web::Json<SeedReqJson>,
) -> Result<(), Error> {
    let db = &mut web_data
        .pool
        .get()
        .map_err(|e| handle_error(e, "failed to get db connection"))?;

    let torrent_name =
        get_torrent_name_from_url(&item.seed_url).unwrap_or_else(|| item.seed_url.clone());

    let qb = web_data.qb.read().await;

    qb.qb_api_del_torrent(&torrent_name)
        .await
        .map_err(|e| handle_error(e, "task_delete, qb_api_del_torrent failed"))?;

    dao::anime_task::delete_anime_task_by_torrent_name(db, &torrent_name)
        .await
        .map_err(|e| {
            handle_error(
                e,
                "task_delete, dao::anime_task::delete_anime_task_by_torrent_name failed",
            )
        })?;
    Ok(())
}

pub async fn task_update(web_data: web::Data<WebData>) -> Result<(), Error> {
    let db = &mut web_data
        .pool
        .get()
        .map_err(|e| handle_error(e, "failed to get db connection"))?;

    let qb = web_data.qb.read().await;
    do_anime_task::update_qb_task_status(&qb, db)
        .await
        .map_err(|e| {
            handle_error(
                e,
                "task_update, do_anime_task::update_qb_task_status failed",
            )
        })?;
    Ok(())
}

async fn search_anime(
    web_data: web::Data<WebData>,
    item: web::Json<AnimeKeyWordReqJson>,
) -> Result<Vec<anime_list::AnimeList>, Error> {
    let db = &mut web_data
        .pool
        .get()
        .map_err(|e| handle_error(e, "failed to get db connection"))?;

    let mut result = dao::anime_list::search_by_anime_name(db, &item.key)
        .await
        .map_err(|e| {
            handle_error(
                e,
                "search_anime, dao::anime_list::search_by_anime_name failed",
            )
        })?;
    result.sort_by(|a, b| b.subscribe_status.cmp(&a.subscribe_status));

    Ok(result)
}

fn convert_json_seed_to_anime_seed(sj: web::Json<SeedReqJson>) -> anime_seed::AnimeSeed {
    anime_seed::AnimeSeed {
        id: None,
        mikan_id: sj.mikan_id,
        subgroup_id: sj.subgroup_id,
        episode: sj.episode,
        seed_name: sj.seed_name.clone(),
        seed_url: sj.seed_url.clone(),
        seed_status: sj.seed_status,
        seed_size: sj.seed_size.clone(),
    }
}

fn reoder_subgroups(subgroup_vec: Vec<AnimeSubgroup>) -> Vec<AnimeSubgroup> {
    let mut subgroup_583 = None; // ANi
    let mut subgroup_382 = None; // 喵萌奶茶屋
    let mut subgroup_370 = None; // LoliHouse
    let mut other_subgroups = Vec::new();

    for subgroup in subgroup_vec {
        match subgroup.subgroup_id {
            583 => subgroup_583 = Some(subgroup),
            382 => subgroup_382 = Some(subgroup),
            370 => subgroup_370 = Some(subgroup),
            _ => other_subgroups.push(subgroup),
        }
    }

    let mut reordered_subgroups = Vec::new();
    if let Some(subgroup) = subgroup_583 {
        reordered_subgroups.push(subgroup);
    }
    if let Some(subgroup) = subgroup_382 {
        reordered_subgroups.push(subgroup);
    }
    if let Some(subgroup) = subgroup_370 {
        reordered_subgroups.push(subgroup);
    }
    reordered_subgroups.extend(other_subgroups);

    reordered_subgroups
}

fn get_img_name_from_url(img_url: &str) -> Option<String> {
    let parts: Vec<&str> = img_url.split('/').collect();
    if let Some(img_name) = parts.get(4) {
        Some(img_name.to_string())
    } else {
        log::warn!("unexpected img_url format: {}", img_url);
        None
    }
}

fn get_torrent_name_from_url(seed_url: &str) -> Option<String> {
    let parts: Vec<&str> = seed_url.split('/').collect();
    if let Some(torrent_name) = parts.get(3) {
        Some(torrent_name.to_string())
    } else {
        log::warn!("unexpected seed_url format: {}", seed_url);
        None
    }
}

fn convert_spider_subgroup_to_anime_subgroup(
    spider_vec: Vec<spider::Subgroup>,
) -> Vec<anime_subgroup::AnimeSubgroupJson> {
    spider_vec
        .into_iter()
        .map(|s| anime_subgroup::AnimeSubgroupJson {
            subgroup_name: s.subgroup_name,
            subgroup_id: s.subgroup_id,
        })
        .collect()
}

pub async fn get_anime_seed_by_spider(
    mikan_id: i32,
    subgroup_id: i32,
    anime_type: i32,
    mikan: &spider::Mikan,
) -> Result<Vec<anime_seed::AnimeSeedJson>, Error> {
    let seed_list: Vec<spider::Seed> = mikan.get_seed(mikan_id, subgroup_id, anime_type).await?;
    Ok(convert_spider_seed_to_anime_seed(seed_list))
}

fn convert_spider_seed_to_anime_seed(
    spider_vec: Vec<spider::Seed>,
) -> Vec<anime_seed::AnimeSeedJson> {
    spider_vec
        .into_iter()
        .map(|s| anime_seed::AnimeSeedJson {
            mikan_id: s.mikan_id,
            subgroup_id: s.subgroup_id,
            episode: s.episode,
            seed_name: s.seed_name,
            seed_url: s.seed_url,
            seed_status: s.seed_status,
            seed_size: s.seed_size,
        })
        .collect()
}
