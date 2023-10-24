use std::collections::{HashMap, HashSet};
use actix_web::web;
use anyhow::Error;
use crate::mods::spider::Mikan;
use crate::Pool;
use crate::dao;
use crate::models::anime_seed::AnimeSeed;
use crate::models::anime_task::AnimeTaskJson;
use crate::mods::anime_filter;
use crate::mods::qb_api::QbitTaskExecutor;
use diesel::r2d2::PooledConnection;
use diesel::r2d2::ConnectionManager;
use diesel::SqliteConnection;


#[allow(dead_code)]
struct DoAnimeTask {
    pub db_connection: PooledConnection<ConnectionManager<SqliteConnection>>,
    pub subcribe_mikan_id_set: HashSet<i32>,
    pub mikan_id_to_name: HashMap<i32, String>,
    pub subgrouop_id_to_name: HashMap<i32, String>,
    pub anime_task_info_map: HashMap<(i32, i32), AnimeSeed>
}

#[allow(dead_code)]
impl DoAnimeTask {
    pub async fn new(
        pool: web::Data<Pool>
    ) -> Result<Self, Error> {
        let db_connection = pool.get().unwrap();
        Ok(DoAnimeTask{
            db_connection,
            subcribe_mikan_id_set: HashSet::new(),
            mikan_id_to_name: HashMap::new(),
            subgrouop_id_to_name: HashMap::new(),
            anime_task_info_map: HashMap::new()
        })
    } 

    pub async fn create_anime_task(&mut self) -> Result<(), Error>{
        // 取出订阅的全部番剧列表
        let subcribe_anime_list = dao::anime_list::get_by_subscribestatus(&mut self.db_connection, 1).await.unwrap();
        println!("{:?}", subcribe_anime_list);
        
        // 得到订阅的全部种子
        let mut subcribe_anime_seeds: Vec<AnimeSeed> = Vec::new();
        for item in subcribe_anime_list {
            self.subcribe_mikan_id_set.insert(item.mikan_id);
            self.mikan_id_to_name.insert(item.mikan_id, item.anime_name);

            let ret_anime_seeds = dao::anime_seed::get_anime_seed_by_mikan_id(&mut self.db_connection, item.mikan_id).await.unwrap();
            for anime_seed in ret_anime_seeds {
                subcribe_anime_seeds.push(anime_seed);
            }
        }

        // 过滤出新种子
        let exists_anime_task_set = dao::anime_task::get_exist_anime_task_set(&mut self.db_connection).await.unwrap();
        println!("exists_anime_task_set: {:?}", exists_anime_task_set);
        let new_anime_seed_vec = anime_filter::filter_anime_bulk(subcribe_anime_seeds, exists_anime_task_set).await.unwrap();
        println!("new_anime_seed_vec: {:?}", new_anime_seed_vec);

        // 下载种子
        let mikan = Mikan::new().unwrap();
        let mut download_success_vec: Vec<AnimeSeed> = Vec::new();
        let mut download_failed_vec: Vec<AnimeSeed> = Vec::new();

        for new_anime_seed in new_anime_seed_vec {
            match mikan.download_seed(&new_anime_seed.seed_url, &format!("{}{}", "downloads/seed/", new_anime_seed.mikan_id))
                    .await
            {
                Ok(_) => download_success_vec.push(new_anime_seed),
                Err(_) => download_failed_vec.push(new_anime_seed)
            }
        }

        println!("download_failed_vec: {:?}", download_failed_vec);

        // 更新 anime_seed
        let mut anime_task_info_vec: Vec<AnimeTaskJson> = Vec::new();
        for anime_seed in download_success_vec {
            dao::anime_seed::update_anime_seed_status(&mut self.db_connection, anime_seed.seed_name).await.unwrap();
            
            anime_task_info_vec.push(
                AnimeTaskJson { 
                    mikan_id: anime_seed.mikan_id, 
                    episode: anime_seed.episode, 
                    torrent_name: anime_seed.seed_url
                                    .rsplit("/")
                                    .next()
                                    .unwrap_or(&anime_seed.seed_url)
                                    .to_string(),
                    qb_task_status: 0 })
        };

        // 插入 anime_task
        dao::anime_task::add_bulk(&mut self.db_connection, &anime_task_info_vec).await.unwrap();

        // 添加到qb
        let qb_task_executor = QbitTaskExecutor::new_with_login(
            "admin".to_string(), 
            "adminadmin".to_string())
            .await
            .unwrap();

        for anime_task in anime_task_info_vec {
            let anime_name = &self.mikan_id_to_name[&anime_task.mikan_id];
            let subgroup_name = &"test".to_string();
            qb_task_executor.qb_api_add_torrent(&anime_name, &anime_task).await.unwrap();
            qb_task_executor.qb_api_torrent_rename_file(&anime_name, subgroup_name, &anime_task).await.unwrap();
        }
        
        Ok(())
    }

    pub async fn do_anime_task() {
        // spider_task
        // create_anime_task
        // update_qb_task_status
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    pub async fn test() {
        dotenv::dotenv().ok();
        let database_url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set");
        let database_pool = Pool::builder()
            .build(ConnectionManager::<SqliteConnection>::new(database_url))
            .expect("Failed to create pool.");

        let r = DoAnimeTask::new(web::Data::new(database_pool)).await.unwrap();
        println!("{:?}", r.subcribe_mikan_id_set);
        println!("{:?}", r.mikan_id_to_name);
        println!("{:?}", r.subgrouop_id_to_name);
    }

    #[tokio::test]
    pub async fn test_create_anime_task() {
        dotenv::dotenv().ok();
        let database_url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set");
        let database_pool = Pool::builder()
            .build(ConnectionManager::<SqliteConnection>::new(database_url))
            .expect("Failed to create pool.");

        let mut do_anime_task = DoAnimeTask::new(web::Data::new(database_pool)).await.unwrap();

        let _r = do_anime_task.create_anime_task().await.unwrap();

    }
}