use std::collections::HashSet;
use diesel::r2d2::{PooledConnection, ConnectionManager};
use diesel::{RunQueryDsl, delete};
use diesel::dsl::{insert_into, update};
use diesel::prelude::*;
use crate::models::anime_filter::*;
use crate::schema::anime_filter::dsl::*;

// global
#[allow(dead_code)]
pub async fn add_global_subgroup_filter( 
    query_subgroup: i32,
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
) -> Result<AnimeFilter, diesel::result::Error> {
    match anime_filter
        .filter(object.eq(&1))
        .filter(filter_val.eq(&query_subgroup))
        .first::<AnimeFilter>(db_connection) {
        Ok(result) => Ok(result),
        Err(_) => {
            let new_anime_filer = PostAnimeFilter {
                mikan_id    : &0,
                filter_type : &"subgroup",
                filter_val  : &query_subgroup,
                object      : &1
            };
            insert_into(anime_filter)
                .values(&new_anime_filer)
                .execute(db_connection)
                .expect("Error saving new anime seed");
            let result = anime_filter
                .order(id.desc())
                .first(db_connection)
                .unwrap();
            Ok(result)
        }
    }
}

// global
#[allow(dead_code)]
pub async fn delete_global_subgroup_filter_by_subgroup_id(
    query_subgroup: i32,
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
) -> Result<usize, diesel::result::Error> {
    Ok(
        delete(anime_filter
            .filter(object.eq(1))
            .filter(filter_val.eq(&query_subgroup))
            .filter(filter_type.eq(&"subgroup")))
            .execute(db_connection)
            .expect("Error delete global subgroup filter")
    )
}

// global
#[allow(dead_code)]
pub async fn get_global_subgroup_filter_set(
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
) -> HashSet<i32> {
    let mut subgroup_set: HashSet<i32> = HashSet::new();
    if let Ok(result_vec)  = anime_filter
        .filter(filter_type.eq(&"subgroup"))
        .filter(object.eq(&1))
        .load::<AnimeFilter>(db_connection) {
        for result in result_vec {
            subgroup_set.insert(result.filter_val);
        }
    }
    subgroup_set
}



// local 
#[allow(dead_code)]
pub async fn add_local_subgroup_filter_by_mikan_id(
    query_mikan_id: i32,
    query_subgroup: i32,
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
) -> Result<(), diesel::result::Error> {
    match anime_filter
    .filter(mikan_id.eq(query_mikan_id))
    .filter(filter_type.eq(&"subgroup"))
    .filter(filter_val.eq(&query_subgroup))
    .filter(object.eq(&0))
    .first::<AnimeFilter>(db_connection) {
    Ok(_) => Ok(()),
    Err(_) => {
        let new_anime_filer = PostAnimeFilter {
            mikan_id    : &query_mikan_id,
            filter_type : &"subgroup",
            filter_val  : &query_subgroup,
            object      : &0
        };
        insert_into(anime_filter)
            .values(&new_anime_filer)
            .execute(db_connection)
            .expect("Error saving new anime seed");
        Ok(())}
    }
}

// local
#[allow(dead_code)]
pub async fn delete_local_subgroup_filter_by_mikan_id(
    quary_mikan_id: i32,
    query_subgroup: i32,
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>
) -> Result<usize, diesel::result::Error>{
    Ok(
        delete(anime_filter
            .filter(mikan_id.eq(&quary_mikan_id))
            .filter(filter_type.eq(&"subgroup")))
            .filter(filter_val.eq(&query_subgroup))
            .filter(object.eq(0))
            .execute(db_connection)
            .expect("Error delete global subgroup filter")
    )
}

// local 
#[allow(dead_code)]
pub async fn get_local_subgroup_filter_set_by_mikan_id(
    quary_mikan_id: i32,
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
) -> Result<HashSet<i32>, diesel::result::Error> {
    let mut subgroup_set: HashSet<i32> = HashSet::new();
    if let Ok(result_vec)  = anime_filter
        .filter(mikan_id.eq(&quary_mikan_id))
        .filter(filter_type.eq(&"subgroup"))
        .filter(object.eq(&0))
        .load::<AnimeFilter>(db_connection) {
        for result in result_vec {
            subgroup_set.insert(result.filter_val);
        }
    }
    Ok(subgroup_set)
}

// local 
#[allow(dead_code)]
pub async fn add_local_episode_filter_by_mikan_id(
    query_mikan_id: i32,
    query_episode: i32,
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
) -> Result<(), diesel::result::Error> {
    match anime_filter
    .filter(mikan_id.eq(&query_mikan_id))
    .filter(filter_type.eq(&"episode"))
    .filter(object.eq(&0))
    .first::<AnimeFilter>(db_connection) {
    Ok(_) => {
        update(anime_filter
                        .filter(mikan_id.eq(&query_mikan_id))
                        .filter(filter_type.eq(&"episode"))
                        .filter(object.eq(&0)))
                        .set(filter_val.eq(query_episode))
                        .execute(db_connection)
                        .expect("update failed");
        Ok(())
    },
    Err(_) => {
        let new_anime_filer = PostAnimeFilter {
            mikan_id    : &query_mikan_id,
            filter_type : &"episode",
            filter_val  : &query_episode,
            object      : &0
        };
        insert_into(anime_filter)
            .values(&new_anime_filer)
            .execute(db_connection)
            .expect("Error saving new anime seed");
        Ok(())}
    }
}

// local
#[allow(dead_code)]
pub async fn delete_local_episode_filter_by_mikan_id(
    quary_mikan_id: i32,
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>
) -> Result<usize, diesel::result::Error>{
    Ok(
        delete(anime_filter
            .filter(mikan_id.eq(&quary_mikan_id))
            .filter(filter_type.eq(&"episode")))
            .filter(object.eq(0))
            .execute(db_connection)
            .expect("Error delete global subgroup filter")
    )
}

// local 
#[allow(dead_code)]
pub async fn get_local_episode_filter_by_mikan_id(
    quary_mikan_id: &i32,
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
) -> Result<i32, diesel::result::Error> {
    match anime_filter.filter(mikan_id.eq(&quary_mikan_id))
                        .filter(filter_type.eq(&"subgroup"))
                        .filter(object.eq(&0))
                        .first::<AnimeFilter>(db_connection) 
    {
        Ok(result) => {
            Ok(result.filter_val)
        }
        Err(_) => Ok(0)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use diesel::r2d2::ConnectionManager;
    use crate::Pool;
    use actix_web::web;

    #[tokio::test]
    async fn test() {
        dotenv::dotenv().ok();
        let database_url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set");
        let database_pool = Pool::builder()
            .build(ConnectionManager::<SqliteConnection>::new(database_url))
            .expect("Failed to create pool.");
        
        let pool = web::Data::new(database_pool);
        let db_connection = &mut pool.get().unwrap();

        add_local_subgroup_filter_by_mikan_id(114, 514, db_connection).await.unwrap();
        add_local_subgroup_filter_by_mikan_id(114, 114514, db_connection).await.unwrap();
        add_local_subgroup_filter_by_mikan_id(114, 1919810, db_connection).await.unwrap();

        delete_local_subgroup_filter_by_mikan_id(114, 514, db_connection).await.unwrap();
    }
}