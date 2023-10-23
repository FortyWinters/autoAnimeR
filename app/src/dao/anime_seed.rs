use actix::fut::result;
use actix_web::web;
use anyhow::Result;
use diesel::{RunQueryDsl, delete};
use diesel::dsl::insert_into;
use diesel::prelude::*;
use crate::Pool;
use crate::models::anime_seed::*;
use crate::schema::anime_seed::dsl::*;

// insert single data into anime_list
#[allow(dead_code)]
pub async fn add(
    pool: web::Data<Pool>,
    item: AnimeSeedJson
) -> Result<AnimeSeed, diesel::result::Error> {
    let db_connection = &mut pool.get().unwrap();
    match anime_seed
        .filter(seed_url.eq(&item.seed_url))
        .first::<AnimeSeed>(db_connection) {
        Ok(result) => Ok(result),
        Err(_) => {
            let new_anime_seed = PostAnimeSeed {
                mikan_id    : &item.mikan_id,
                subgroup_id : &item.subgroup_id,
                episode     : &item.episode,
                seed_name   : &item.seed_name,
                seed_url    : &item.seed_url,
                seed_status : &item.seed_status,
                seed_size   : &item.seed_size
            };
            insert_into(anime_seed)
                .values(&new_anime_seed)
                .execute(db_connection)
                .expect("Error saving new anime seed");
            let result = anime_seed.order(id.desc())
                .first(db_connection)
                .unwrap();
            Ok(result)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use diesel::r2d2::ConnectionManager;
    use crate::Pool;
    use actix_web::web;

    #[tokio::test]
    async fn test_add () {
        dotenv::dotenv().ok();
        let database_url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set");
        let database_pool = Pool::builder()
            .build(ConnectionManager::<SqliteConnection>::new(database_url))
            .expect("Failed to create pool.");
        
        let pool = web::Data::new(database_pool);
        let test_anime_seed_json = AnimeSeedJson {
            mikan_id: 123,
            subgroup_id: 456,
            episode: 1,
            seed_name: "test_seed_name".to_string(),
            seed_url: "test_seed_url".to_string(),
            seed_status: 0,
            seed_size: "test_seed_size".to_string()
        };

        add(pool, test_anime_seed_json).await.unwrap();
    }
}