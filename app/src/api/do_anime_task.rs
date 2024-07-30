use crate::api::spider_task::do_spider_task;
use crate::models::anime_list::{AnimeList, AnimeListJson};
use crate::models::anime_seed::AnimeSeed;
use crate::models::anime_task::{AnimeTask, AnimeTaskJson};
use crate::mods::config::Config;
use crate::mods::{anime_filter, qb_api::QbitTaskExecutor, spider::Mikan, video_proccessor};
use crate::v2::anime::AnimeRequestJson;
use crate::{dao, v2};

use anyhow::Error;
use chrono::prelude::*;
use diesel::r2d2::{ConnectionManager, PooledConnection};
use diesel::SqliteConnection;
use futures::future::join_all;
use log;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{prelude::*, Read, Write};
use std::sync::Arc;
use tokio::sync::RwLock as TokioRwLock;
use tokio::time::{self, sleep, Duration};

pub fn handle_error<E: std::fmt::Debug>(e: E, message: &str) -> anyhow::Error {
    log::error!("{}, error: {:?}", message, e);
    Error::msg("Internal server error")
}

#[allow(dead_code)]
pub enum DownloadSeedStatus {
    SUCCESS(AnimeSeed),
    FAILED(AnimeSeed),
}

#[allow(dead_code)]
pub async fn create_anime_task_bulk(
    mikan: &Mikan,
    qb_task_executor: &QbitTaskExecutor,
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
) -> Result<(), Error> {
    if !qb_task_executor.is_login {
        log::warn!("qbittorrent client not started");
        return Ok(());
    }

    // 取出订阅的全部番剧列表
    let anime_list_vec = dao::anime_list::get_by_subscribestatus(db_connection, 1)
        .await
        .unwrap();
    log::debug!("anime list: {:?}", anime_list_vec);

    // 得到订阅的全部种子
    let mut anime_seed_map: HashMap<i32, Vec<AnimeSeed>> = HashMap::new();
    for anime_list in anime_list_vec {
        let ret_anime_seeds =
            dao::anime_seed::get_anime_seed_by_mikan_id(db_connection, anime_list.mikan_id)
                .await
                .unwrap();
        anime_seed_map.insert(anime_list.mikan_id, ret_anime_seeds);
    }

    // 过滤并下载
    filter_and_download(&mikan, qb_task_executor, db_connection, anime_seed_map)
        .await
        .unwrap();

    Ok(())
}

#[allow(dead_code)]
pub async fn create_anime_task_single(
    mikan: &Mikan,
    qb_task_executor: &QbitTaskExecutor,
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
    mikan_id: i32,
    episode: i32, // anime_task_idx
) -> Result<(), Error> {
    if !qb_task_executor.is_login {
        log::warn!("qbittorrent client not started");
        return Ok(());
    }

    let anime_seed_vec =
        dao::anime_seed::get_by_mikanid_and_episode(db_connection, mikan_id, episode)
            .await
            .unwrap();

    let anime_seed_map = vec![(mikan_id, anime_seed_vec)].into_iter().collect();

    filter_and_download(&mikan, &qb_task_executor, db_connection, anime_seed_map)
        .await
        .unwrap();

    Ok(())
}

#[allow(dead_code)]
pub async fn create_anime_task_by_seed(
    mikan: &Mikan,
    anime_seed: AnimeSeed,
    qb_task_executor: &QbitTaskExecutor,
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
) -> Result<(), Error> {
    match download_seed_handler(anime_seed, mikan).await.unwrap() {
        DownloadSeedStatus::SUCCESS(anime_seed) => {
            let anime_task_info = AnimeTaskJson {
                mikan_id: anime_seed.mikan_id.clone(),
                episode: anime_seed.episode.clone(),
                torrent_name: anime_seed
                    .seed_url
                    .rsplit("/")
                    .next()
                    .unwrap_or(&anime_seed.seed_url)
                    .to_string(),
                qb_task_status: 0,
                rename_status: 0,
                filename: "".to_string(),
            };

            match create_qb_task(&qb_task_executor, db_connection, &anime_seed).await {
                Ok(_) => {
                    dao::anime_task::add(db_connection, &anime_task_info)
                        .await
                        .unwrap();
                    Ok(())
                }
                Err(e) => Err(e),
            }
        }
        DownloadSeedStatus::FAILED(_) => Err(Error::msg("Failed to download seed.")),
    }
}

pub async fn get_video_config(
    download_path: &String,
) -> Result<(File, HashMap<String, VideoConfig>), Error> {
    let video_config_path = format!("{}/.videoConfig.json", download_path);

    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(video_config_path)
        .map_err(|e| handle_error(e, "Failed to create video config file."))?;

    let mut contents = String::new();
    let _ = file.read_to_string(&mut contents);

    if contents.trim().is_empty() {
        contents = "{}".to_string();
    }

    let video_config: HashMap<String, VideoConfig> = serde_json::from_str(&contents)
        .map_err(|e| handle_error(e, "Failed to convert vedio config file from bytes to json"))?;
    log::debug!("{:?}", video_config);

    Ok((file, video_config))
}

#[allow(dead_code)]
pub async fn create_anime_task_from_exist_files(
    video_file_lock: &Arc<TokioRwLock<bool>>,
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
    qb: &Arc<TokioRwLock<QbitTaskExecutor>>,
    config: &Arc<TokioRwLock<Config>>,
) -> Result<(), Error> {
    let video_file_lock = video_file_lock.read().await;

    let download_path = {
        let qb = qb.read().await;
        qb.qb_api_get_download_path().await.unwrap()
    };

    let (_, video_config) = get_video_config(&download_path)
        .await
        .map_err(|e| handle_error(e, "Failed to get video config"))?;
    drop(video_file_lock); // Explicitly drop the lock here

    let files = std::fs::read_dir(&download_path)?;

    for file in files {
        let file = file?;
        let filename = file.file_name().to_string_lossy().to_string();

        if ["seed", ".DS_Store", ".videoConfig.json", "images"].contains(&filename.as_str()) {
            continue;
        }

        let mikan_id = match Regex::new(r"\((\d+)\)")?.captures(&filename) {
            Some(captures) => captures.get(1).unwrap().as_str().parse::<i32>().unwrap(),
            None => {
                log::info!("Failed to get mikan id, file name: [{}]", filename);
                continue;
            }
        };

        // Update anime list
        let mikan = Mikan::new().unwrap();
        let anime = mikan.get_anime_by_mikan_id(mikan_id).await.unwrap();
        let img_url = anime.img_url.split("?").next().unwrap().to_string();

        dao::anime_list::add(
            db_connection,
            AnimeListJson {
                anime_name: anime.anime_name.clone(),
                anime_type: anime.anime_type,
                mikan_id: anime.mikan_id,
                update_day: anime.update_day,
                img_url: img_url.clone(),
                subscribe_status: anime.subscribe_status,
                bangumi_id: -1,
                bangumi_rank: "".to_string(),
                bangumi_summary: "".to_string(),
                website: "".to_string(),
                anime_status: -1,
                total_episodes: -1,
            },
        )
        .await?;

        // Download image
        let save_path = {
            let config = config.read().await;
            config.img_path.clone()
        };
        mikan.download_img(&img_url, &save_path).await.unwrap();

        // Update anime seed
        if v2::anime::seed_update(db_connection, AnimeRequestJson { mikan_id })
            .await
            .is_err()
        {
            log::warn!(
                "Failed to update seed for anime: {}, retrying once.",
                anime.anime_name
            );
            if v2::anime::seed_update(db_connection, AnimeRequestJson { mikan_id })
                .await
                .is_err()
            {
                log::warn!(
                    "Failed to update seed for anime: {}, please retry later.",
                    anime.anime_name
                );
                continue;
            }
        }

        for video in file.path().read_dir()? {
            let video = video?.file_name().to_string_lossy().to_string();
            let parts: Vec<&str> = video.split(" - ").collect();
            if parts.len() == 1 {
                log::info!("File name error, {}", parts[0]);
                continue;
            }

            let extension = video.split('.').last().unwrap();
            if ["ass", "vtt"].contains(&extension) {
                continue;
            }

            let cur_video_config = match video_config.get(&video).ok_or("error") {
                Ok(cfg) => cfg,
                Err(_) => {
                    log::warn!(
                        "Failed to get anime info from video config file, anime name: [{}]",
                        video
                    );
                    continue;
                }
            };

            let anime_task = AnimeTaskJson {
                mikan_id,
                episode: cur_video_config.episode,
                torrent_name: cur_video_config.torrent_name.clone(),
                qb_task_status: 1,
                rename_status: 1,
                filename: video.clone(),
            };

            if let Err(e) = dao::anime_task::add(db_connection, &anime_task).await {
                log::error!(
                    "Failed to add new anime task to db: {:?}, error: {:?}",
                    anime_task,
                    e
                );
            } else {
                log::info!(
                    "Added new anime task to db, anime_task detail: {:?}",
                    anime_task
                );
            }

            match dao::anime_seed::update_anime_seed_status(
                db_connection,
                &cur_video_config.torrent_name,
            )
            .await
            {
                Ok(_) => log::info!("Successfully updated anime seed for [{}]", mikan_id),
                Err(e) => log::error!(
                    "Failed to update anime seed status for [{}]: {:?}",
                    mikan_id,
                    e
                ),
            }
        }
    }

    Ok(())
}

#[allow(dead_code)]
pub async fn filter_and_download(
    mikan: &Mikan,
    qb_task_executor: &QbitTaskExecutor,
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
    anime_seed_map: HashMap<i32, Vec<AnimeSeed>>,
) -> Result<(), Error> {
    let anime_task_set = dao::anime_task::get_exist_anime_task_set(db_connection)
        .await
        .unwrap();

    // 过滤出新种子
    let new_anime_seed_vec = anime_filter::filter_v3(db_connection, anime_seed_map, anime_task_set)
        .await
        .unwrap();

    log::debug!("new anime seed: {:?}", new_anime_seed_vec);
    // println!("new_anime_seed_vec: {:?}", new_anime_seed_vec);

    // 下载种子
    let mut download_success_vec: Vec<AnimeSeed> = Vec::new();
    let mut download_failed_vec: Vec<AnimeSeed> = Vec::new();

    if new_anime_seed_vec.len() > 0 {
        let task_res_vec = join_all(
            new_anime_seed_vec
                .into_iter()
                .map(|anime_seed| download_seed_handler(anime_seed, &mikan)),
        )
        .await;

        for task_res in task_res_vec {
            match task_res {
                Ok(status) => match status {
                    DownloadSeedStatus::SUCCESS(anime_seed) => {
                        download_success_vec.push(anime_seed)
                    }
                    DownloadSeedStatus::FAILED(anime_seed) => download_failed_vec.push(anime_seed),
                },
                Err(_) => continue,
            }
        }
    }
    log::info!("download failed vec: {:?}", download_failed_vec);
    // println!("download_failed_vec: {:?}", download_failed_vec);

    // 更新 anime_seed table
    let mut anime_task_info_vec: Vec<AnimeTaskJson> = Vec::new();
    for anime_seed in &download_success_vec {
        dao::anime_seed::update_anime_seed_status(db_connection, &anime_seed.seed_url)
            .await
            .unwrap();

        anime_task_info_vec.push(AnimeTaskJson {
            mikan_id: anime_seed.mikan_id.clone(),
            episode: anime_seed.episode.clone(),
            torrent_name: anime_seed
                .seed_url
                .rsplit("/")
                .next()
                .unwrap_or(&anime_seed.seed_url)
                .to_string(),
            qb_task_status: 0,
            rename_status: 0,
            filename: "".to_string(),
        })
    }

    // 插入 anime_task
    dao::anime_task::add_bulk(db_connection, &anime_task_info_vec)
        .await
        .unwrap();

    // 添加到qb
    for anime_seed in &download_success_vec {
        create_qb_task(&qb_task_executor, db_connection, anime_seed)
            .await
            .unwrap();
    }
    Ok(())
}

#[allow(dead_code)]
pub async fn create_qb_task(
    qb_task_executor: &QbitTaskExecutor,
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
    anime_seed: &AnimeSeed,
) -> Result<(), Error> {
    let anime_name = dao::anime_list::get_by_mikanid(db_connection, anime_seed.mikan_id.clone())
        .await
        .unwrap()
        .anime_name;

    match qb_task_executor
        .qb_api_add_torrent(&anime_name, &anime_seed)
        .await
    {
        Ok(_) => Ok(()),
        Err(e) => {
            log::warn!("failed to create qb task, err: {}", e);
            Err(Error::from(e))
        }
    }
}

// This is a ugly failure.
#[allow(dead_code)]
pub async fn update_qb_task_status(
    qb_task_executor: &QbitTaskExecutor,
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
) -> Result<(), Error> {
    if let Ok(fn_task_vec) = qb_task_executor.qb_api_completed_torrent_list().await {
        for fn_task in fn_task_vec {
            dao::anime_task::update_qb_task_status(db_connection, fn_task)
                .await
                .unwrap();
        }
    } else {
        log::warn!("failed to get finished torrent")
    }
    Ok(())
}

pub async fn download_seed_handler(
    anime_seed: AnimeSeed,
    mikan: &Mikan,
) -> Result<DownloadSeedStatus, Error> {
    log::info!("processing {}", anime_seed.seed_name);
    // println!("processing {}", anime_seed.seed_name);
    match mikan
        .download_seed(
            &anime_seed.seed_url,
            &format!("{}{}", "downloads/seed/", anime_seed.mikan_id),
        )
        .await
    {
        Ok(_) => Ok(DownloadSeedStatus::SUCCESS(anime_seed)),
        Err(_) => Ok(DownloadSeedStatus::FAILED(anime_seed)),
    }
}

#[allow(dead_code)]
pub async fn get_under_update_task_list(
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
) -> Result<(), Error> {
    let new_sub_annime_vec =
        dao::anime_list::get_by_subscribe_and_anime_status(&1, &-1, db_connection)
            .await
            .unwrap();

    let mut subscribed_anime_vec =
        dao::anime_list::get_by_subscribe_and_anime_status(&1, &0, db_connection)
            .await
            .unwrap();

    subscribed_anime_vec.extend(new_sub_annime_vec);

    let weekday = (Local::now().weekday().num_days_from_monday() + 1) as i32;

    println!("Today is: {}", weekday);

    let under_update_task_list: Vec<AnimeList> = subscribed_anime_vec
        .into_iter()
        .filter(|anime_list| {
            let update_day = anime_list.update_day;
            update_day == weekday || update_day - 1 == weekday || update_day + 1 == weekday
        })
        .collect();

    for task in under_update_task_list {
        println!("{:?}", task.anime_name);
    }

    Ok(())
}

#[allow(dead_code)]
pub async fn run(
    qb_task_executor: &Arc<TokioRwLock<QbitTaskExecutor>>,
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
) {
    // spider_task
    let mikan = Mikan::new().unwrap();
    let subscribed_anime_vec = dao::anime_list::get_by_subscribestatus(db_connection, 1)
        .await
        .unwrap();

    let st_anime_vec = do_spider_task(&mikan, subscribed_anime_vec, db_connection).await;
    let _new_seed_vec = dao::anime_seed::add_bulk_with_response(db_connection, st_anime_vec)
        .await
        .unwrap()
        .success_vec;

    log::debug!("Create anime task start");
    let qb = qb_task_executor.read().await;
    create_anime_task_bulk(&mikan, &qb, db_connection)
        .await
        .unwrap();
    drop(qb);
    log::debug!("Create anime task done");
}

#[allow(dead_code)]
pub async fn run_task(
    status: &Arc<TokioRwLock<bool>>,
    qb_task_executor: &Arc<TokioRwLock<QbitTaskExecutor>>,
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
) {
    let interval = 120;
    {
        let mut writer = status.write().await;
        *writer = true;
    }
    log::debug!("Start scheduled task");
    loop {
        let reader = status.read().await;
        let val = *reader;
        drop(reader);

        if val {
            log::debug!("Running scheduled task with interval 2 min");
            run(qb_task_executor, db_connection).await;
            time::sleep(Duration::from_secs(interval)).await;
        } else {
            break;
        }
    }
}

#[allow(dead_code)]
pub async fn exit_task(status: &Arc<TokioRwLock<bool>>) {
    log::debug!("Stop scheduled task");
    let mut writer = status.write().await;
    *writer = false;
    log::debug!("Task status has been changed to false");
    // println!("{}", writer);
}

pub async fn change_task_interval(
    interval: i32,
    status: &Arc<TokioRwLock<bool>>,
    qb_task_executor: &Arc<TokioRwLock<QbitTaskExecutor>>,
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
) {
    let interval_sec = interval as u64 * 60;
    {
        let mut writer = status.write().await;
        *writer = true;
    }
    log::debug!("Start scheduled task");
    loop {
        let reader = status.read().await;
        let val = *reader;
        drop(reader);

        if val {
            log::debug!("Running scheduled task with interval: {} min", interval);
            run(qb_task_executor, db_connection).await;
            time::sleep(Duration::from_secs(interval_sec)).await;
        } else {
            break;
        }
    }
}

#[allow(dead_code)]
pub async fn get_task_status(status: &Arc<TokioRwLock<bool>>) -> Result<bool, Error> {
    let reader = status.read().await;
    let val = *reader;
    drop(reader);
    Ok(val)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VideoConfig {
    pub torrent_name: String,
    pub mikan_id: i32,
    pub episode: i32,
    pub subtitle_nb: i32,
    pub subtitle: Vec<String>,
}

#[allow(dead_code)]
pub async fn auto_update_rename_extract(
    video_file_lock: &Arc<TokioRwLock<bool>>,
    pool: &diesel::r2d2::Pool<ConnectionManager<diesel::SqliteConnection>>,
    qb_task_executor: &Arc<TokioRwLock<QbitTaskExecutor>>,
) -> Result<(), Error> {
    log::info!("Start auto rename and update thread");
    loop {
        {
            let mut db_connection = pool.get().unwrap();
            let nb_new_finished_task = auto_update_handler(&qb_task_executor, &mut db_connection)
                .await
                .map_err(|e| handle_error(e, "Failed to get finished task"))?;

            if nb_new_finished_task > 0 {
                auto_rename_and_extract_handler(
                    &video_file_lock,
                    &qb_task_executor,
                    &mut db_connection,
                )
                .await
                .map_err(|e| handle_error(e, "Failed to execute rename task"))?;
            }
        }
        sleep(Duration::from_secs(5)).await;
    }
}

pub async fn auto_update_handler(
    qb_task_executor: &Arc<TokioRwLock<QbitTaskExecutor>>,
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
) -> Result<i32, Error> {
    let qb = qb_task_executor.read().await;
    let finished_task_set = qb
        .qb_api_completed_torrent_set()
        .await
        .map_err(|e| handle_error(e, "Failed to get finished task list"))?;
    let under_update_task_list = dao::anime_task::get_by_qbtaskstatus(db_connection, 0)
        .await
        .map_err(|e| handle_error(e, "Failed to get under update task list"))?;

    let mut task_cnt = 0;

    for task in under_update_task_list {
        if finished_task_set.contains(&task.torrent_name) {
            // println!("{}", task.torrent_name);
            dao::anime_task::update_qb_task_status(db_connection, task.torrent_name.to_string())
                .await
                .map_err(|e| handle_error(e, "failed to access anime_task table"))?;
            task_cnt += 1;
            log::info!("update torrent: {} download status", task.torrent_name);
        }
    }
    Ok(task_cnt)
}

#[allow(dead_code)]
pub async fn auto_rename_and_extract_handler(
    video_file_lock: &Arc<TokioRwLock<bool>>,
    qb_task_executor: &Arc<TokioRwLock<QbitTaskExecutor>>,
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
) -> Result<(), Error> {
    let _guard = video_file_lock.write().await;
    let qb = qb_task_executor.read().await;
    let download_path = qb.qb_api_get_download_path().await.unwrap();

    let (mut file, mut video_config) = get_video_config(&download_path)
        .await
        .map_err(|e| handle_error(e, "Failed to get video config"))?;

    let task_list = dao::anime_task::get_by_task_status(db_connection, 1, 0)
        .await
        .map_err(|e| handle_error(e, "Failed to get anime task by task status."))?;

    log::debug!("{:?}", task_list);

    // rename -> extract -> write VideoConfig
    for task in task_list {
        // rename
        if let Ok((cur_file_name, cur_total_file_path)) =
            rename_file(&qb, db_connection, &task).await
        {
            dao::anime_task::update_task_status(
                db_connection,
                &task.torrent_name,
                1,
                1,
                &cur_file_name,
            )
            .await
            .map_err(|e| {
                handle_error(
                    e,
                    format!("Failed to update task status for anime_task: {:?}", task).as_str(),
                )
            })?;

            // extract subtitles
            let mut subtitle_vec: Vec<String> = vec![];
            let extension = cur_file_name.split(".").last().unwrap();
            if extension == "mkv" || extension == "mp4" {
                if let Ok(res) = video_proccessor::extract_subtitle(&cur_total_file_path).await {
                    subtitle_vec = res;
                } else {
                    log::warn!("");
                }
            }

            // write VideoConfig
            let cur_config = VideoConfig {
                torrent_name: task.torrent_name.clone(),
                mikan_id: task.mikan_id,
                episode: task.episode,
                subtitle_nb: subtitle_vec.len() as i32,
                subtitle: subtitle_vec,
            };
            video_config.insert(cur_file_name, cur_config);
            qb.qb_api_del_torrent(&task.torrent_name)
                .await
                .map_err(|e| {
                    handle_error(
                        e,
                        format!("Failed to delete task for qb: {:?}", task).as_str(),
                    )
                })?;
        } else {
            log::info!("Failed to execute rename task for anime_task: {:?}", task);
        }
    }

    file.seek(std::io::SeekFrom::Start(0)).unwrap();
    file.set_len(0).unwrap();
    file.write_all(
        serde_json::to_string_pretty(&video_config)
            .unwrap()
            .as_bytes(),
    )
    .map_err(|e| handle_error(e, "Failed to update video config file."))?;
    Ok(())
}

#[allow(dead_code)]
pub async fn rename_file(
    qb_task_executor: &QbitTaskExecutor,
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
    anime_task: &AnimeTask,
) -> Result<(String, String), Error> {
    let path = qb_task_executor
        .qb_api_get_download_path()
        .await
        .map_err(|e| {
            handle_error(
                e,
                "Faile to get download path, please check your qbittorrent client config",
            )
        })?;

    let anime_name = dao::anime_list::get_by_mikanid(db_connection, anime_task.mikan_id)
        .await
        .map_err(|e| handle_error(e, "Failed to get anime name."))?
        .anime_name;

    // println!("{:?}", anime_task.torrent_name);
    let file_name = qb_task_executor
        .qb_api_torrent_info(&anime_task.torrent_name)
        .await
        .map_err(|e| handle_error(e, "Failed to get original video name."))?
        .name;
    if file_name.len() == 0 {
        return Err(Error::msg("Failed to get original video name."));
    }

    // Total name: path/anime_name(mikan_id)/video_name.mp4
    let total_path = format!(
        "{}/{}({})/{}",
        path, anime_name, anime_task.mikan_id, file_name
    );
    log::debug!("total_path: {}", total_path);

    let extension = match file_name.rsplit('.').next() {
        Some(ext) => ext,
        None => "mp4",
    };
    log::debug!("extension: {}", extension);

    let quary_item = format!("%{}", anime_task.torrent_name);
    let subgroup_id = dao::anime_seed::get_anime_seed_by_seed_url(db_connection, &quary_item)
        .await
        .map_err(|e| handle_error(e, "Failed to get subgroup_id"))?
        .subgroup_id;

    let subgroup = dao::anime_subgroup::get_by_subgroupid(db_connection, &subgroup_id)
        .await
        .map_err(|e| handle_error(e, "Failed to get subgroup name"))?
        .subgroup_name;
    if file_name.len() == 0 {
        return Err(Error::msg("Failed to get subgroup name."));
    }

    let new_file_name = format!(
        "{} - {} - {}.{}",
        anime_name, anime_task.episode, subgroup, extension
    );
    let new_total_path = format!(
        "{}/{}({})/{}",
        path, anime_name, anime_task.mikan_id, new_file_name
    );

    log::info!(
        "old file name: {}, new file name: {}",
        total_path,
        new_total_path
    );

    let _ = fs::rename(&total_path, &new_total_path);

    Ok((new_file_name, new_total_path))
}

#[allow(dead_code)]
pub async fn add_default_filter(
    config: &Arc<TokioRwLock<Config>>,
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
) -> Result<(), Error> {
    let config = config.read().await;

    for sub in &config.anime_config.subgroup_filter.preference {
        let _ = dao::anime_filter::add_global_subgroup_filter(sub, db_connection)
            .await
            .unwrap();
    }

    for sub in &config.anime_config.subgroup_filter.avoid {
        let _ = dao::anime_filter::add_global_subgroup_filter(&-sub, db_connection)
            .await
            .unwrap();
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::Config;
    use crate::Pool;

    #[tokio::test]
    pub async fn test() {
        dotenv::dotenv().ok();
        let config = Config::load_config("./config/config.yaml").await.unwrap();

        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let database_pool = Pool::builder()
            .build(ConnectionManager::<SqliteConnection>::new(database_url))
            .expect("Failed to create pool.");

        let _qb = Arc::new(TokioRwLock::new(
            QbitTaskExecutor::new_with_config(&config)
                .await
                .expect("Failed to create qb client"),
        ));

        // let video_file_lock = Arc::new(TokioRwLock::new(false));
        // let _ =
        //     auto_update_rename_extract(&video_file_lock, &mut database_pool.get().unwrap(), &qb)
        //         .await;
        let _ = get_under_update_task_list(&mut database_pool.get().unwrap())
            .await
            .unwrap();
    }
}
