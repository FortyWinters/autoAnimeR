use crate::models::anime_task::*;
use crate::schema::anime_task::dsl::*;
use diesel::dsl::{insert_into, update};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, PooledConnection};
use diesel::{delete, RunQueryDsl};
use std::collections::HashSet;

// insert single data into anime_task
#[allow(dead_code)]
pub async fn add(
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
    item: &AnimeTaskJson,
) -> Result<AnimeTask, diesel::result::Error> {
    match anime_task
        .filter(torrent_name.eq(&item.torrent_name))
        .first::<AnimeTask>(db_connection)
    {
        Ok(result) => Ok(result),
        Err(_) => {
            let new_anime_task = PostAnimeTask {
                mikan_id: &item.mikan_id,
                episode: &item.episode,
                torrent_name: &item.torrent_name,
                qb_task_status: &item.qb_task_status,
                rename_status: &item.rename_status,
                filename: &item.filename,
                is_new: &item.is_new,
            };
            insert_into(anime_task)
                .values(&new_anime_task)
                .execute(db_connection)
                .expect("Error saving new anime seed");
            let result = anime_task.order(id.desc()).first(db_connection).unwrap();
            Ok(result)
        }
    }
}

#[allow(dead_code)]
pub async fn add_bulk(
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
    item_vec: &Vec<AnimeTaskJson>,
) -> Result<i32, diesel::result::Error> {
    let mut success_num: i32 = 0;

    for item in item_vec {
        if let Err(_) = anime_task
            .filter(torrent_name.eq(&item.torrent_name))
            .first::<AnimeTask>(db_connection)
        {
            let new_anime_task = PostAnimeTask {
                mikan_id: &item.mikan_id,
                episode: &item.episode,
                torrent_name: &item.torrent_name,
                qb_task_status: &item.qb_task_status,
                rename_status: &item.rename_status,
                filename: &item.filename,
                is_new: &item.is_new
            };
            insert_into(anime_task)
                .values(&new_anime_task)
                .execute(db_connection)
                .expect("Error saving new anime seed");
            success_num += 1;
        }
    }
    Ok(success_num)
}

#[allow(dead_code)]
pub async fn get_exist_anime_task_by_mikan_id(
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
    item: i32, // mikan_id
) -> Result<Vec<AnimeTask>, diesel::result::Error> {
    match anime_task
        .filter(mikan_id.eq(&item))
        .load::<AnimeTask>(db_connection)
    {
        Ok(result) => Ok(result),
        Err(e) => Err(e),
    }
}

#[allow(dead_code)]
pub async fn get_exist_anime_task_by_torrent_name(
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
    item: String, // mikan_id
) -> Result<Vec<AnimeTask>, diesel::result::Error> {
    match anime_task
        .filter(torrent_name.eq(&item))
        .load::<AnimeTask>(db_connection)
    {
        Ok(result) => Ok(result),
        Err(e) => Err(e),
    }
}

#[allow(dead_code)]
pub async fn delete_anime_task_by_mikan_id(
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
    item: i32, // mikan_id
) -> Result<(), diesel::result::Error> {
    let _r = delete(anime_task.filter(mikan_id.eq(&item)))
        .execute(db_connection)
        .expect("Error deleting anime_task");
    Ok(())
}

#[allow(dead_code)]
pub async fn delete_anime_task_by_torrent_name(
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
    item: &String, // torrent_name
) -> Result<(), diesel::result::Error> {
    let _r = delete(anime_task.filter(torrent_name.like(&item)))
        .execute(db_connection)
        .expect("Error deleting anime_task");
    Ok(())
}

#[allow(dead_code)]
pub async fn delete_anime_task_by_mikan_id_and_episode(
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
    query_mikan_id: i32,
    query_episode: i32,
) -> Result<(), diesel::result::Error> {
    let _r = delete(anime_task.filter(mikan_id.eq(&query_mikan_id)))
        .filter(episode.eq(&query_episode))
        .execute(db_connection)
        .expect("Error deleting anime_task");
    Ok(())
}

#[allow(dead_code)]
pub async fn delete_all(
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
) -> Result<(), diesel::result::Error> {
    let _r = delete(anime_task)
        .execute(db_connection)
        .expect("Error deleting anime_task");
    Ok(())
}

#[allow(dead_code)]
pub async fn update_qb_task_status(
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
    item: String, // torrent_name
) -> Result<(), diesel::result::Error> {
    if let Ok(_) = anime_task
        .filter(torrent_name.like(&item))
        .first::<AnimeTask>(db_connection)
    {
        update(anime_task.filter(torrent_name.like(&item)))
            .set(qb_task_status.eq(1))
            .execute(db_connection)
            .expect("save failed");
    }
    Ok(())
}

#[allow(dead_code)]
pub async fn update_task_status(
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
    item: &String, // torrent_name
    new_qb_status: i32,
    new_rename_status: i32,
    new_filename: &String,
    isnew: i32,
) -> Result<(), diesel::result::Error> {
    if let Ok(_) = anime_task
        .filter(torrent_name.like(&item))
        .first::<AnimeTask>(db_connection)
    {
        update(anime_task.filter(torrent_name.like(&item)))
            .set((
                qb_task_status.eq(new_qb_status),
                rename_status.eq(new_rename_status),
                filename.eq(new_filename),
                is_new.eq(isnew),
            ))
            .execute(db_connection)
            .expect("save failed");
    }
    Ok(())
}

// query all data from anime_task
pub async fn get_all(
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
) -> Result<Vec<AnimeTask>, diesel::result::Error> {
    let result: Vec<AnimeTask> = anime_task.load::<AnimeTask>(db_connection)?;
    Ok(result)
}

pub async fn get_exist_anime_task_set(
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
) -> Result<HashSet<(i32, i32)>, diesel::result::Error> {
    let result: Vec<AnimeTask> = anime_task.load::<AnimeTask>(db_connection).unwrap();
    let mut exist_anime_task_set: HashSet<(i32, i32)> = HashSet::new();
    for item in result {
        exist_anime_task_set.insert((item.mikan_id, item.episode));
    }
    Ok(exist_anime_task_set)
}

#[allow(dead_code)]
pub async fn get_exist_anime_task_set_by_mikanid(
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
    query_mikan_id: i32,
) -> Result<HashSet<(i32, i32)>, diesel::result::Error> {
    let result: Vec<AnimeTask> = anime_task
        .filter(mikan_id.eq(query_mikan_id))
        .load::<AnimeTask>(db_connection)
        .unwrap();
    let mut exist_anime_task_set: HashSet<(i32, i32)> = HashSet::new();
    for item in result {
        exist_anime_task_set.insert((item.mikan_id, item.episode));
    }
    Ok(exist_anime_task_set)
}

pub async fn get_by_qbtaskstatus(
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
    query_qbtaskstatus: i32,
) -> Result<Vec<AnimeTask>, diesel::result::Error> {
    let result: Vec<AnimeTask> = anime_task
        .filter(qb_task_status.eq(query_qbtaskstatus))
        .load::<AnimeTask>(db_connection)?;
    Ok(result)
}

#[allow(dead_code)]
pub async fn get_by_task_status(
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
    query_qbtask_status: i32,
    query_rename_status: i32,
) -> Result<Vec<AnimeTask>, diesel::result::Error> {
    let result: Vec<AnimeTask> = anime_task
        .filter(qb_task_status.eq(query_qbtask_status))
        .filter(rename_status.eq(query_rename_status))
        .load::<AnimeTask>(db_connection)?;
    Ok(result)
}

#[allow(dead_code)]
pub async fn get_by_torrent_name(
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
    query_torrent_name: &String,
) -> Result<AnimeTask, diesel::result::Error> {
    let result: AnimeTask = anime_task
        .filter(torrent_name.eq(&query_torrent_name))
        .first::<AnimeTask>(db_connection)?;
    Ok(result)
}

#[allow(dead_code)]
pub async fn update_isnew_status(
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
    item: &String, // torrent_name
    isnew: i32,
) -> Result<(), diesel::result::Error> {
    if let Ok(_) = anime_task
        .filter(torrent_name.like(&item))
        .first::<AnimeTask>(db_connection)
    {
        update(anime_task.filter(torrent_name.like(&item)))
            .set((
                is_new.eq(isnew),
            ))
            .execute(db_connection)
            .expect("save failed");
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::Pool;
    use actix_web::web;
    use diesel::r2d2::ConnectionManager;

    #[tokio::test]
    async fn test_add() {
        dotenv::dotenv().ok();
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let database_pool = Pool::builder()
            .build(ConnectionManager::<SqliteConnection>::new(database_url))
            .expect("Failed to create pool.");

        let pool = web::Data::new(database_pool);
        let db_connection = &mut pool.get().unwrap();

        let test_anime_task_json = AnimeTaskJson {
            mikan_id: 3061,
            episode: 1,
            torrent_name: "test_torrent_name".to_string(),
            qb_task_status: 0,
            rename_status: 0,
            filename: "test_file_name".to_string(),
            is_new: 0,
        };

        add(db_connection, &test_anime_task_json).await.unwrap();
    }

    #[tokio::test]
    async fn test_add_bulk() {
        dotenv::dotenv().ok();
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let database_pool = Pool::builder()
            .build(ConnectionManager::<SqliteConnection>::new(database_url))
            .expect("Failed to create pool.");

        let pool = web::Data::new(database_pool);
        let db_connection = &mut pool.get().unwrap();
        let test_anime_task_json = vec![
            AnimeTaskJson {
                mikan_id: 3061,
                episode: 1,
                torrent_name: "test_torrent_name_1".to_string(),
                qb_task_status: 0,
                rename_status: 0,
                filename: "test_file_name_1".to_string(),
                is_new: 1,
            },
            AnimeTaskJson {
                mikan_id: 114514,
                episode: 1919810,
                torrent_name: "test_torrent_name_2".to_string(),
                qb_task_status: 0,
                rename_status: 0,
                filename: "test_file_name_2".to_string(),
                is_new: 1,
            },
        ];
        add_bulk(db_connection, &test_anime_task_json)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_get_exist_anime_task() {
        dotenv::dotenv().ok();
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let database_pool = Pool::builder()
            .build(ConnectionManager::<SqliteConnection>::new(database_url))
            .expect("Failed to create pool.");

        let pool = web::Data::new(database_pool);
        let db_connection = &mut pool.get().unwrap();
        let r = get_exist_anime_task_by_mikan_id(db_connection, 123)
            .await
            .unwrap();
        // let r = get_exist_anime_task_by_torrent_name(pool, "test_torrent_name".to_string()).await.unwrap();
        println!("{:?}", r);
    }

    #[tokio::test]
    async fn test_update_qb_task_status() {
        dotenv::dotenv().ok();
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let database_pool = Pool::builder()
            .build(ConnectionManager::<SqliteConnection>::new(database_url))
            .expect("Failed to create pool.");

        let pool = web::Data::new(database_pool);
        let db_connection = &mut pool.get().unwrap();

        let r = update_qb_task_status(db_connection, "test_torrent_name".to_string())
            .await
            .unwrap();
        // let r = get_exist_anime_task_by_torrent_name(pool, "test_torrent_name".to_string()).await.unwrap();
        println!("{:?}", r);
    }

    #[tokio::test]
    async fn test_delete_anime_task() {
        dotenv::dotenv().ok();
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let database_pool = Pool::builder()
            .build(ConnectionManager::<SqliteConnection>::new(database_url))
            .expect("Failed to create pool.");

        let pool = web::Data::new(database_pool);
        let db_connection = &mut pool.get().unwrap();
        // let _r = delete_anime_task_by_torrent_name(pool, "test_torrent_name".to_string()).await.unwrap();
        let _r = delete_anime_task_by_mikan_id_and_episode(db_connection, 3143, 3)
            .await
            .unwrap();
    }
}
