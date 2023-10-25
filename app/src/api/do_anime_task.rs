use std::collections::HashSet;
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
    pub qb_task_executor: QbitTaskExecutor,
    pub db_connection: PooledConnection<ConnectionManager<SqliteConnection>>
}

#[allow(dead_code)]
impl DoAnimeTask {
    pub async fn new(
        pool: web::Data<Pool>,
        qb_task_executor: QbitTaskExecutor
    ) -> Result<Self, Error> {
        let db_connection = pool.get().unwrap();
        Ok(DoAnimeTask{
            qb_task_executor,
            db_connection
        })
    } 

    pub async fn create_anime_task_bulk(&mut self) -> Result<(), Error>{
        // 取出订阅的全部番剧列表
        let anime_list_vec = dao::anime_list::get_by_subscribestatus(&mut self.db_connection, 1).await.unwrap();
        println!("{:?}", anime_list_vec);
        
        // 得到订阅的全部种子
        let mut anime_seed_vec: Vec<AnimeSeed> = Vec::new();
        for anime_list in anime_list_vec {
            let ret_anime_seeds = dao::anime_seed::get_anime_seed_by_mikan_id(&mut self.db_connection, anime_list.mikan_id).await.unwrap();
            for anime_seed in ret_anime_seeds {
                anime_seed_vec.push(anime_seed);
            }
        }

        // 过滤并下载
        let mut anime_task_set = dao::anime_task::get_exist_anime_task_set(&mut self.db_connection).await.unwrap();
        self.filter_and_download(anime_seed_vec, &mut anime_task_set).await.unwrap();
        
        Ok(())
    }

    pub async fn create_anime_task_single(
        &mut self, 
        mikan_id: i32, 
        episode: i32 // anime_task_idx
    ) -> Result<(), Error> {
        let anime_seed_vec = dao::anime_seed::get_by_mikanid_and_episode(
            &mut self.db_connection, 
            mikan_id,
            episode)
            .await
            .unwrap();
        
        let mut anime_task_set = dao::anime_task::get_exist_anime_task_set_by_mikanid(
            &mut self.db_connection, 
            mikan_id)
            .await
            .unwrap();

        self.filter_and_download(anime_seed_vec, &mut anime_task_set).await.unwrap();

        Ok(())
    }

    pub async fn filter_and_download (
        &mut self,
        anime_seed_vec: Vec<AnimeSeed>,
        anime_task_set: &mut HashSet<(i32, i32)>,
    ) -> Result<(), Error> {
         
         // 过滤出新种子
         let new_anime_seed_vec = anime_filter::filter_anime_bulk(anime_seed_vec, anime_task_set).await.unwrap();
         println!("new_anime_seed_vec: {:?}", new_anime_seed_vec);
 
         // 下载种子
         let mikan = Mikan::new().unwrap();
         let mut download_success_vec: Vec<AnimeSeed> = Vec::new();
         let mut download_failed_vec: Vec<AnimeSeed> = Vec::new();
 
         if new_anime_seed_vec.len() > 0 {
             for new_anime_seed in new_anime_seed_vec {
                println!("processing {}", new_anime_seed.seed_name);
                match mikan.download_seed(&new_anime_seed.seed_url, &format!("{}{}", "downloads/seed/", new_anime_seed.mikan_id)).await
                {
                    Ok(()) => download_success_vec.push(new_anime_seed),
                    Err(_) => download_failed_vec.push(new_anime_seed)
                }
             }
         }

         println!("download_failed_vec: {:?}", download_failed_vec);
 
         // 更新 anime_seed table
         let mut anime_task_info_vec: Vec<AnimeTaskJson> = Vec::new();
         for anime_seed in &download_success_vec {
             dao::anime_seed::update_anime_seed_status(&mut self.db_connection, &anime_seed.seed_url).await.unwrap();
             
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
         dao::anime_task::add_bulk(&mut self.db_connection, &anime_task_info_vec).await.unwrap();
 
         // 添加到qb
         for anime_seed in &download_success_vec {
            self.create_qb_task(anime_seed).await.unwrap();
         }

        Ok(())
    }

    pub async fn create_qb_task(
        &mut self,
        anime_seed: &AnimeSeed
    ) -> Result<(), Error> {
        let anime_name = dao::anime_list::get_by_mikanid(&mut self.db_connection, anime_seed.mikan_id.clone())
            .await
            .unwrap()
            .anime_name;

        let subgroup_name = dao::anime_subgroup::get_by_subgroupid(&mut self.db_connection, &anime_seed.subgroup_id)
            .await
            .unwrap()
            .subgroup_name;

        self.qb_task_executor.qb_api_add_torrent(&anime_name, &anime_seed).await.unwrap();
        self.qb_task_executor.qb_api_torrent_rename_file(&anime_name, &subgroup_name, &anime_seed).await.unwrap();
        Ok(())
    }

    pub async fn run(&self) {
        // spider_task
        // create_anime_task_bulk
        // update_qb_task_status
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::api::anime;

    #[tokio::test]
    pub async fn test() {
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

        let _r = DoAnimeTask::new(web::Data::new(database_pool), qb_task_executor).await.unwrap();
    }

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

        let mut do_anime_task = DoAnimeTask::new(web::Data::new(database_pool), qb_task_executor).await.unwrap();
        
        // let test_anime_seed_json = AnimeSeedJson {
        //     mikan_id: 3143,
        //     subgroup_id: 382,
        //     episode: 3,
        //     seed_name: "【喵萌奶茶屋】★10月新番★[米基与达利 / Migi to Dali][03][1080p][简日双语][招募翻译]".to_string(),
        //     seed_url: "/Download/20231021/55829bc76527a4868f9fd5c40e769f618f30e85b.torrent".to_string(),
        //     seed_status: 0,
        //     seed_size: "349.4MB".to_string()
        // };

        // let test_anime_subgroup = AnimeSubgroupJson {
        //     subgroup_id: 382,
        //     subgroup_name: "喵萌奶茶屋".to_string()
        // };

        // reset 
        dao::anime_seed::delete_all(&mut do_anime_task.db_connection).await.unwrap();
        dao::anime_task::delete_all(&mut do_anime_task.db_connection).await.unwrap();
        
        let anime_list_vec = dao::anime_list::get_by_subscribestatus(&mut do_anime_task.db_connection, 1).await.unwrap();

        for anime_list in &anime_list_vec {
            let _r = anime::get_anime_seed(anime_list.mikan_id, &mut do_anime_task.db_connection).await.unwrap();
        }

        // let _r = do_anime_task.create_anime_task_bulk().await.unwrap();
        // let _r = do_anime_task.create_anime_task_bulk().await.unwrap();

    }
}