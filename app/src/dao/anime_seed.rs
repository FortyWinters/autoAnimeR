use actix_web::web;
use chrono::format::Item;
use diesel::{RunQueryDsl, delete};
use diesel::dsl::{insert_into, update};
use diesel::prelude::*;
use diesel::r2d2::{PooledConnection, ConnectionManager};
use crate::Pool;
use crate::models::anime_seed::*;
use crate::schema::anime_seed::dsl::*;

// insert single data into anime_seed
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
            let result = anime_seed
                .order(id.desc())
                .first(db_connection)
                .unwrap();
            Ok(result)
        }
    }
}

#[allow(dead_code)]
pub async fn add_bulk(
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
    item_vec: Vec<AnimeSeedJson>
) -> Result<i32, diesel::result::Error> {
    let mut success_num: i32 = 0;

    for item in &item_vec {
        if let Err(_) = anime_seed.filter(seed_url.eq(&item.seed_url)).first::<AnimeSeed>(db_connection) {
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
                .expect("save failed");
            success_num += 1;
        }
    }
    Ok(success_num)
}

#[allow(dead_code)]
pub async fn update_anime_seed_status(
    pool: web::Data<Pool>,
    item: String // seed_url
) -> Result<(), diesel::result::Error> {
    let db_connection = &mut pool.get().unwrap();
    if let Ok(_) = anime_seed.filter(seed_url.eq(&item)).first::<AnimeSeed>(db_connection) {
        update(anime_seed.filter(seed_url.eq(&item)))
            .set(seed_status.eq(1))
            .execute(db_connection)
            .expect("save failed");
    }
    Ok(())
}

#[allow(dead_code)]
pub async fn get_anime_seed_by_mikan_id(
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
    item: i32 // mikan_id
) -> Result<Vec<AnimeSeed>, diesel::result::Error> {
    match anime_seed.filter(mikan_id.eq(&item)).load::<AnimeSeed>(db_connection) {
        Ok(result) => Ok(result),
        Err(e) => Err(e)
    }
}

#[allow(dead_code)]
pub async fn get_anime_seed_by_seed_url(
    pool: web::Data<Pool>,
    item: String // seed_url
) -> Result<AnimeSeed, diesel::result::Error> {
    let db_connection = &mut pool.get().unwrap();
    match anime_seed.filter(seed_url.eq(&item)).first::<AnimeSeed>(db_connection) {
        Ok(result) => Ok(result),
        Err(e) => Err(e)
    }
}

#[allow(dead_code)]
pub async fn delete_anime_seed_by_mikan_id(
    pool: web::Data<Pool>,
    item: i32 // mikan_id
) -> Result<(), diesel::result::Error> {
    let db_connection = &mut pool.get().unwrap();
    let _r = delete(anime_seed.filter(mikan_id.eq(&item)))
        .execute(db_connection)
        .expect("Error deleting anime_task");
    Ok(())
}

#[allow(dead_code)]
pub async fn delete_anime_seed_by_seed_url(
    pool: web::Data<Pool>,
    item: String // torrent_name
) -> Result<(), diesel::result::Error> {
    let db_connection = &mut pool.get().unwrap();
    let _r = delete(anime_seed.filter(seed_url.like(&item)))
        .execute(db_connection)
        .expect("Error deleting anime_task");
    Ok(())
}

pub async fn get_by_mikanid_subgeoupid(
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
    query_mikanid: i32,
    query_subgroup: i32
) -> Result<Vec<AnimeSeed>, diesel::result::Error> {
    match anime_seed
        .filter(mikan_id.eq(&query_mikanid))
        .filter(subgroup_id.eq(&query_subgroup))
        .load::<AnimeSeed>(db_connection) {
        Ok(result) => Ok(result),
        Err(e) => Err(e)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use diesel::r2d2::ConnectionManager;
    use crate::Pool;
    use actix_web::web;

    #[tokio::test]
    async fn test_add() {
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

    #[tokio::test]
    async fn test_add_bulk() {
        dotenv::dotenv().ok();
        let database_url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set");
        let database_pool = Pool::builder()
            .build(ConnectionManager::<SqliteConnection>::new(database_url))
            .expect("Failed to create pool.");
        
        let pool = web::Data::new(database_pool);
        let test_anime_seed_json = vec![
            AnimeSeedJson {
                mikan_id: 123,
                subgroup_id: 456,
                episode: 1,
                seed_name: "test_seed_name_0".to_string(),
                seed_url: "test_seed_url_0".to_string(),
                seed_status: 0,
                seed_size: "test_seed_size_0".to_string()
            },
            AnimeSeedJson {
                mikan_id: 123,
                subgroup_id: 1919810,
                episode: 1,
                seed_name: "test_seed_name_1".to_string(),
                seed_url: "test_seed_url_1".to_string(),
                seed_status: 0,
                seed_size: "test_seed_size_1".to_string()
            }];
        let db_connection = &mut pool.get().unwrap();
        add_bulk(db_connection, test_anime_seed_json).await.unwrap();
    }

    #[tokio::test]
    async fn test_update_anime_seed_status() {
        dotenv::dotenv().ok();
        let database_url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set");
        let database_pool = Pool::builder()
            .build(ConnectionManager::<SqliteConnection>::new(database_url))
            .expect("Failed to create pool.");
        
        let pool = web::Data::new(database_pool);
        let _r = update_anime_seed_status(pool, "test_seed_url_1".to_string()).await.unwrap();
    }

    #[tokio::test]
    async fn test_get_anime_seed() {
        dotenv::dotenv().ok();
        let database_url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set");
        let database_pool = Pool::builder()
            .build(ConnectionManager::<SqliteConnection>::new(database_url))
            .expect("Failed to create pool.");
        
        let pool = web::Data::new(database_pool);
        let db_connection = &mut pool.get().unwrap();
        let r = get_anime_seed_by_mikan_id(db_connection, 123).await.unwrap();
        // let r = get_anime_seed_by_seed_url(pool, "test_seed_url_1".to_string()).await.unwrap();
        println!("{:?}", r);
    }

    #[tokio::test]
    async fn test_delete_anime_seed() {
        dotenv::dotenv().ok();
        let database_url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set");
        let database_pool = Pool::builder()
            .build(ConnectionManager::<SqliteConnection>::new(database_url))
            .expect("Failed to create pool.");
        
        let pool = web::Data::new(database_pool);
        //let _r = delete_anime_seed_by_seed_url(pool, "test_seed_url".to_string()).await.unwrap();
        let _r = delete_anime_seed_by_mikan_id(pool, 123).await.unwrap();
    }
}