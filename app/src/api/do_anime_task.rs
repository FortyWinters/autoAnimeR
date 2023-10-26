use std::collections::HashSet;
use anyhow::Error;
use crate::mods::spider::Mikan;
use crate::dao;
use crate::models::anime_seed::AnimeSeed;
use crate::models::anime_task::AnimeTaskJson;
use crate::mods::anime_filter;
use crate::mods::qb_api::QbitTaskExecutor;
use diesel::r2d2::PooledConnection;
use diesel::r2d2::ConnectionManager;
use diesel::SqliteConnection;
use futures::future::join_all;
use log;
use std::sync::Arc;
use tokio::sync::RwLock as TokioRwLock;
use tokio::time::{self, Duration};
use crate::api::spider_task::do_spider_task;

#[allow(dead_code)]
pub enum DownloadSeedStatus {
    SUCCESS(AnimeSeed),
    FAILED(AnimeSeed)
}

#[allow(dead_code)]
pub async fn create_anime_task_bulk(
    mikan: &Mikan,
    qb_task_executor: &QbitTaskExecutor,
    db_connection: & mut PooledConnection<ConnectionManager<SqliteConnection>>
) -> Result<(), Error>{
    // 取出订阅的全部番剧列表
    let anime_list_vec = dao::anime_list::get_by_subscribestatus(db_connection, 1).await.unwrap();
    println!("{:?}", anime_list_vec);
    
    // 得到订阅的全部种子
    let mut anime_seed_vec: Vec<AnimeSeed> = Vec::new();
    for anime_list in anime_list_vec {
        let ret_anime_seeds = dao::anime_seed::get_anime_seed_by_mikan_id(db_connection, anime_list.mikan_id).await.unwrap();
        for anime_seed in ret_anime_seeds {
            anime_seed_vec.push(anime_seed);
        }
    }

    // 过滤并下载
    let mut anime_task_set = dao::anime_task::get_exist_anime_task_set(db_connection).await.unwrap();
    filter_and_download(
        &mikan,
        qb_task_executor,
        db_connection,
        anime_seed_vec, 
        &mut anime_task_set).await.unwrap();
    
    Ok(())
}

#[allow(dead_code)]
pub async fn create_anime_task_single(
    mikan: &Mikan,
    qb_task_executor: QbitTaskExecutor,
    db_connection: & mut PooledConnection<ConnectionManager<SqliteConnection>>,
    mikan_id: i32, 
    episode: i32 // anime_task_idx
) -> Result<(), Error> {
    let anime_seed_vec = dao::anime_seed::get_by_mikanid_and_episode(
        db_connection, 
        mikan_id,
        episode)
        .await
        .unwrap();
    
    let mut anime_task_set = dao::anime_task::get_exist_anime_task_set_by_mikanid(
        db_connection, 
        mikan_id)
        .await
        .unwrap();

    filter_and_download(
        &mikan,
        &qb_task_executor,
        db_connection,
        anime_seed_vec, 
        &mut anime_task_set)
        .await
        .unwrap();

    Ok(())
}

#[allow(dead_code)]
pub async fn create_anime_task_by_seed (
    mikan: &Mikan,
    anime_seed: AnimeSeed,
    qb_task_executor: &QbitTaskExecutor,
    db_connection: & mut PooledConnection<ConnectionManager<SqliteConnection>>,
) -> Result<(), Error>{
    match download_seed_handler(anime_seed, mikan).await.unwrap() {
        DownloadSeedStatus::SUCCESS(seed) => {
            Ok(create_qb_task(&qb_task_executor, db_connection, &seed).await.unwrap())
        }
        DownloadSeedStatus::FAILED(_) => Err(Error::msg("message"))
    } 
}


#[allow(dead_code)]
pub async fn filter_and_download (
    mikan: &Mikan,
    qb_task_executor: &QbitTaskExecutor,
    db_connection: & mut PooledConnection<ConnectionManager<SqliteConnection>>,
    anime_seed_vec: Vec<AnimeSeed>,
    anime_task_set: &mut HashSet<(i32, i32)>,
) -> Result<(), Error> {
        
    // 过滤出新种子
    let new_anime_seed_vec = anime_filter::filter_anime_bulk(anime_seed_vec, anime_task_set).await.unwrap();
    println!("new_anime_seed_vec: {:?}", new_anime_seed_vec);

    // 下载种子
    let mut download_success_vec: Vec<AnimeSeed> = Vec::new();
    let mut download_failed_vec: Vec<AnimeSeed> = Vec::new();
    
     if new_anime_seed_vec.len() > 0 {
        let task_res_vec = join_all(new_anime_seed_vec
            .into_iter()
            .map(|anime_seed|{
                download_seed_handler(anime_seed, &mikan)
            })).await;
    
        for task_res in task_res_vec {
            match task_res {
                Ok(status) => {
                    match status {
                        DownloadSeedStatus::SUCCESS(anime_seed) => download_success_vec.push(anime_seed),
                        DownloadSeedStatus::FAILED(anime_seed) => download_failed_vec.push(anime_seed)
                    }
                }
                Err(_) => continue
            }
        }
    }

    println!("download_failed_vec: {:?}", download_failed_vec);

    // 更新 anime_seed table
    let mut anime_task_info_vec: Vec<AnimeTaskJson> = Vec::new();
    for anime_seed in &download_success_vec {
        dao::anime_seed::update_anime_seed_status(db_connection, &anime_seed.seed_url).await.unwrap();
        
        anime_task_info_vec.push(
            AnimeTaskJson { 
                mikan_id       : anime_seed.mikan_id.clone(), 
                episode        : anime_seed.episode.clone(), 
                torrent_name   : anime_seed.seed_url
                                    .rsplit("/")
                                    .next()
                                    .unwrap_or(&anime_seed.seed_url)
                                    .to_string(),
                qb_task_status : 0 
            }
        )
    }

    // 插入 anime_task
    dao::anime_task::add_bulk(db_connection, &anime_task_info_vec).await.unwrap();

    // 添加到qb
    for anime_seed in &download_success_vec {
        create_qb_task(
            &qb_task_executor,
            db_connection,
            anime_seed)
            .await
            .unwrap();
    }
    Ok(())
}

#[allow(dead_code)]
pub async fn create_qb_task(
    qb_task_executor: &QbitTaskExecutor,
    db_connection: & mut PooledConnection<ConnectionManager<SqliteConnection>>,
    anime_seed: &AnimeSeed
) -> Result<(), Error> {
    let anime_name = dao::anime_list::get_by_mikanid(db_connection, anime_seed.mikan_id.clone())
        .await
        .unwrap()
        .anime_name;

    let subgroup_name = dao::anime_subgroup::get_by_subgroupid(db_connection, &anime_seed.subgroup_id)
        .await
        .unwrap()
        .subgroup_name;

    qb_task_executor.qb_api_add_torrent(&anime_name, &anime_seed).await.unwrap();
    qb_task_executor.qb_api_torrent_rename_file(&anime_name, &subgroup_name, &anime_seed).await.unwrap();
    Ok(())
}

pub async fn download_seed_handler(
    anime_seed: AnimeSeed,
    mikan: &Mikan
) -> Result<DownloadSeedStatus, Error> {
    println!("processing {}", anime_seed.seed_name);
    match mikan.download_seed(&anime_seed.seed_url, &format!("{}{}", "downloads/seed/", anime_seed.mikan_id)).await {
        Ok(_) => Ok(DownloadSeedStatus::SUCCESS(anime_seed)),
        Err(_) => Ok(DownloadSeedStatus::FAILED(anime_seed))
    }
}

#[allow(dead_code)]
pub async fn run(
    qb_task_executor: &QbitTaskExecutor,
    db_connection: & mut PooledConnection<ConnectionManager<SqliteConnection>>,
) {
    // spider_task
    let mikan = Mikan::new().unwrap();
    let subscribed_anime_vec = dao::anime_list::get_by_subscribestatus(db_connection, 1).await.unwrap();

    let st_anime_vec = do_spider_task(&mikan, subscribed_anime_vec, db_connection).await;
    let new_seed_vec = dao::anime_seed::add_bulk_with_response(db_connection, st_anime_vec).await.unwrap();

    if new_seed_vec.sucess_vec.len() > 0 {
        log::info!("Create anime task start");
        create_anime_task_bulk(&mikan, qb_task_executor, db_connection).await.unwrap();
        log::info!("Create anime task done");
    }
    
    // update_qb_task_status
}

#[allow(dead_code)]
pub async fn run_task(
    status: &Arc<TokioRwLock<bool>>,
    qb_task_executor: &QbitTaskExecutor,
    db_connection: & mut PooledConnection<ConnectionManager<SqliteConnection>>,
) {
    let interval = 120;
    {
        let mut writer = status.write().await;
        *writer = true;
    }
    log::info!("Start scheduled task");
    loop {
        let reader = status.read().await;
        let val = *reader;
        drop(reader);

        if val {
            log::info!("Running scheduled task");
            run(qb_task_executor, db_connection).await;
            time::sleep(Duration::from_secs(interval)).await;
        } else {
            break;
        }
    }
}

#[allow(dead_code)]
pub async fn exit_task(status: &Arc<TokioRwLock<bool>>) {
    log::info!("Stop scheduled task");
    let mut writer = status.write().await;
    *writer = false;
    println!("{}", writer);
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::api::anime;
    use crate::Pool;

    #[tokio::test]
    pub async fn test_create_anime_task() {
        dotenv::dotenv().ok();
        let database_url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set");
        let database_pool = Pool::builder()
            .build(ConnectionManager::<SqliteConnection>::new(database_url))
            .expect("Failed to create pool.");
        
        let qb_task_executor = QbitTaskExecutor::new_with_login(
            "admin".to_string(), 
            "adminadmin".to_string())
            .await
            .unwrap();

        let db_connection = &mut database_pool.get().unwrap();
        let mikan = Mikan::new().unwrap();
        // reset 
        dao::anime_seed::delete_all(db_connection).await.unwrap();
        dao::anime_task::delete_all(db_connection).await.unwrap();
        
        let anime_list_vec = dao::anime_list::get_by_subscribestatus(db_connection, 1).await.unwrap();

        for anime_list in &anime_list_vec {
            let _r = anime::get_anime_seed(anime_list.mikan_id, db_connection).await.unwrap();
        }

        let _r = create_anime_task_bulk(&mikan, &qb_task_executor, db_connection).await.unwrap();

    }
}