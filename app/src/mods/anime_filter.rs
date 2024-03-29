use crate::dao;
use crate::models::anime_seed::AnimeSeed;
use anyhow::Error;
use diesel::r2d2::ConnectionManager;
use diesel::r2d2::PooledConnection;
use diesel::SqliteConnection;
use std::collections::HashSet;

#[allow(dead_code)]
pub async fn filter_anime_bulk(
    anime_seed_vec: Vec<AnimeSeed>,
    anime_task_set: &mut HashSet<(i32, i32)>,
) -> Result<Vec<AnimeSeed>, Error> {
    let mut new_anime_seed_vec: Vec<AnimeSeed> = Vec::new();

    for anime_seed in anime_seed_vec {
        if anime_seed.seed_status == 1
            || anime_task_set.contains(&(anime_seed.mikan_id, anime_seed.episode))
        {
            continue;
        }
        anime_task_set.insert((anime_seed.mikan_id, anime_seed.episode));
        new_anime_seed_vec.push(anime_seed)
    }
    Ok(new_anime_seed_vec)
}

pub async fn filter_anime_bulk_with_anime_filter(
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
    anime_seed_vec: Vec<AnimeSeed>,
    anime_task_set: &mut HashSet<(i32, i32)>,
) -> Result<Vec<AnimeSeed>, Error> {
    let mut new_anime_seed_vec: Vec<AnimeSeed> = Vec::new();
    let global_subgroup_filter =
        dao::anime_filter::get_global_subgroup_filter_set(db_connection).await;

    for anime_seed in anime_seed_vec {
        let local_subgroup_filter = dao::anime_filter::get_local_subgroup_filter_set_by_mikan_id(
            anime_seed.mikan_id,
            db_connection,
        )
        .await
        .unwrap();
        let local_episode_filter = dao::anime_filter::get_local_episode_filter_by_mikan_id(
            &anime_seed.mikan_id,
            db_connection,
        )
        .await
        .unwrap();

        if anime_seed.seed_status == 1
            || anime_task_set.contains(&(anime_seed.mikan_id, anime_seed.episode))
            || global_subgroup_filter.contains(&anime_seed.subgroup_id)
            || local_subgroup_filter.contains(&anime_seed.subgroup_id)
            || anime_seed.episode < local_episode_filter
        {
            log::info!("skip torrent: {}", anime_seed.seed_name);
            continue;
        }
        anime_task_set.insert((anime_seed.mikan_id, anime_seed.episode));
        new_anime_seed_vec.push(anime_seed)
    }
    Ok(new_anime_seed_vec)
}
