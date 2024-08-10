use crate::dao;
use crate::models;
use crate::mods::spider;
use diesel::r2d2::{ConnectionManager, PooledConnection};
use diesel::SqliteConnection;
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct SpiderTaskAnime {
    pub mikan_id: i32,
    pub anime_type: i32,
    pub subgroup_id: i32,
    pub subgroup_name: String,
}

pub async fn do_spider_task(
    mikan: &spider::Mikan,
    subscribed_anime_vec: Vec<models::anime_list::AnimeList>,
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
) -> Vec<models::anime_seed::AnimeSeedJson> {
    let mut st_seed_vec: Vec<models::anime_seed::AnimeSeedJson> = Vec::new();
    let mut st_anime_vec: Vec<SpiderTaskAnime> = Vec::new();

    if !subscribed_anime_vec.is_empty() {
        let task_res_vec = join_all(
            subscribed_anime_vec
                .into_iter()
                .map(|a| get_st_anime(a, &mikan)),
        )
        .await;

        for task_res in task_res_vec {
            match task_res {
                Ok(st_anime) => {
                    st_anime_vec.extend(st_anime);
                }
                Err(_) => {
                    log::debug!("get subgroup failed")
                }
            }
        }
    }

    let mut st_subgroup_map: HashMap<i32, String> = HashMap::new();
    for st_a in &st_anime_vec {
        st_subgroup_map
            .entry(st_a.subgroup_id)
            .or_insert(st_a.subgroup_name.to_string());
    }
    let anime_subgroup_vec: Vec<models::anime_subgroup::AnimeSubgroupJson> = st_subgroup_map
        .into_iter()
        .map(|(id, name)| models::anime_subgroup::AnimeSubgroupJson {
            subgroup_id: id,
            subgroup_name: name,
        })
        .collect();
    dao::anime_subgroup::add_vec(db_connection, anime_subgroup_vec)
        .await
        .unwrap();

    if !st_anime_vec.is_empty() {
        let task_res_vec = join_all(
            st_anime_vec
                .into_iter()
                .map(|st_a| get_st_seed(st_a.mikan_id, st_a.subgroup_id, st_a.anime_type, &mikan)),
        )
        .await;

        for task_res in task_res_vec {
            match task_res {
                Ok(st_seed) => {
                    st_seed_vec.extend(st_seed);
                }
                Err(_) => {
                    log::debug!("get subgroup failed")
                }
            }
        }
    }
    return st_seed_vec;
}

pub async fn get_st_anime(
    a: models::anime_list::AnimeList,
    mikan: &spider::Mikan,
) -> Result<Vec<SpiderTaskAnime>, ()> {
    match mikan.get_subgroup(a.mikan_id).await {
        Ok(s_vec) => Ok(convert_to_spidertaskanime(a, &s_vec)),
        Err(_) => Err(()),
    }
}

pub async fn get_st_seed(
    mikan_id: i32,
    subgroup_id: i32,
    anime_type: i32,
    mikan: &spider::Mikan,
) -> Result<Vec<models::anime_seed::AnimeSeedJson>, ()> {
    match mikan.get_seed(mikan_id, subgroup_id, anime_type).await {
        Ok(seed_vec) => Ok(convert_spiderseed_to_animeseed(&seed_vec)),
        Err(_) => Err(()),
    }
}

pub fn convert_to_spidertaskanime(
    a: models::anime_list::AnimeList,
    s_vec: &Vec<spider::Subgroup>,
) -> Vec<SpiderTaskAnime> {
    let mut st_anime_vec: Vec<SpiderTaskAnime> = Vec::new();
    for s in s_vec {
        st_anime_vec.push(SpiderTaskAnime {
            mikan_id: a.mikan_id,
            anime_type: a.anime_type,
            subgroup_id: s.subgroup_id,
            subgroup_name: s.subgroup_name.to_string(),
        });
    }
    return st_anime_vec;
}

pub fn convert_spiderseed_to_animeseed(
    spider_vec: &Vec<spider::Seed>,
) -> Vec<models::anime_seed::AnimeSeedJson> {
    spider_vec
        .into_iter()
        .map(|s| models::anime_seed::AnimeSeedJson {
            mikan_id: s.mikan_id,
            subgroup_id: s.subgroup_id,
            episode: s.episode,
            seed_name: s.seed_name.to_string(),
            seed_url: s.seed_url.to_string(),
            seed_status: s.seed_status,
            seed_size: s.seed_size.to_string(),
        })
        .collect()
}

#[cfg(test)]
mod test {
    use crate::api::spider_task;
    use crate::dao;
    use crate::mods::spider;
    use crate::Pool;
    use actix_web::web;
    use diesel::r2d2::ConnectionManager;
    use diesel::SqliteConnection;

    #[tokio::test]
    async fn test_spider_task() {
        dotenv::dotenv().ok();
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let database_pool = Pool::builder()
            .build(ConnectionManager::<SqliteConnection>::new(database_url))
            .expect("Failed to create pool.");
        let pool = web::Data::new(database_pool);

        let db_connection = &mut pool.get().unwrap();
        let subscribed_anime_vec = dao::anime_list::get_by_subscribestatus(db_connection, 1)
            .await
            .unwrap();
        let mikan = spider::Mikan::new().unwrap();

        let st_anime_vec =
            spider_task::do_spider_task(&mikan, subscribed_anime_vec, db_connection).await;
        let new_seed_vec = dao::anime_seed::add_bulk_with_response(db_connection, st_anime_vec)
            .await
            .unwrap();
        println!("{:?}", new_seed_vec.success_vec);
    }
}
