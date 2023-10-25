use std::collections::HashSet;
use crate::models::anime_seed::AnimeSeed;
use anyhow::Error;

pub async fn filter_anime_bulk(
    anime_seed_vec: Vec<AnimeSeed>,
    anime_task_set: &mut HashSet<(i32, i32)>,
)-> Result<Vec<AnimeSeed>, Error>{
    let mut new_anime_seed_vec: Vec<AnimeSeed> = Vec::new();

    for anime_seed in anime_seed_vec {
        if anime_seed.seed_status == 1 || anime_task_set.contains(&(anime_seed.mikan_id, anime_seed.episode)) {
            continue;
        }
        anime_task_set.insert((anime_seed.mikan_id, anime_seed.episode));
        new_anime_seed_vec.push(anime_seed)
    }
    Ok(new_anime_seed_vec)
}
