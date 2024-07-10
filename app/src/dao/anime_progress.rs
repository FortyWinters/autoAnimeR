use crate::models::anime_progess::*;
use crate::schema::anime_progress;
use crate::schema::anime_progress::dsl::*;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, PooledConnection};
use diesel::result::Error;
use diesel::sqlite::SqliteConnection;
use diesel::{delete, RunQueryDsl};

async fn add_or_update_anime_progress<'a>(
    item: &'a AnimeProgressJson,
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
    filter_fn: impl FnOnce() -> anime_progress::BoxedQuery<'a, diesel::sqlite::Sqlite> + 'a,
) -> Result<AnimeProgress, Error> {
    let existing_item = filter_fn()
        .first::<AnimeProgress>(db_connection)
        .optional()?;

    match existing_item {
        Some(existing_item) => {
            diesel::update(anime_progress.filter(id.eq(existing_item.id)))
                .set(progress_status.eq(&item.progress_status))
                .execute(db_connection)?;
            Ok(existing_item)
        }
        None => {
            let new_anime_progress = PostAnimeProgress {
                progress_id: &item.progress_id,
                mikan_id: &item.mikan_id,
                episode: &item.episode,
                torrent_name: &item.torrent_name,
                progress_status: &item.progress_status,
            };
            diesel::insert_into(anime_progress)
                .values(&new_anime_progress)
                .execute(db_connection)?;
            let result = anime_progress.order(id.desc()).first(db_connection)?;
            Ok(result)
        }
    }
}

#[allow(dead_code)]
pub async fn add_with_mikan_id_and_episode(
    item: &AnimeProgressJson,
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
) -> Result<AnimeProgress, Error> {
    add_or_update_anime_progress(&item, db_connection, || {
        anime_progress
            .filter(progress_id.eq(&item.progress_id))
            .filter(mikan_id.eq(&item.mikan_id))
            .filter(episode.eq(&item.episode))
            .into_boxed()
    })
    .await
}

#[allow(dead_code)]
pub async fn add_with_torrent_name(
    item: &AnimeProgressJson,
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
) -> Result<AnimeProgress, Error> {
    add_or_update_anime_progress(&item, db_connection, || {
        anime_progress
            .filter(progress_id.eq(&item.progress_id))
            .filter(torrent_name.eq(&item.torrent_name))
            .into_boxed()
    })
    .await
}

#[allow(dead_code)]
pub async fn get_by_mikan_id_and_episode(
    query_progress_id: &String,
    query_mikan_id: &i32,
    query_episode: &i32,
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
) -> Result<AnimeProgress, diesel::result::Error> {
    match anime_progress
        .filter(progress_id.eq(&query_progress_id))
        .filter(mikan_id.eq(&query_mikan_id))
        .filter(episode.eq(&query_episode))
        .first::<AnimeProgress>(db_connection)
    {
        Ok(result) => Ok(result),
        Err(e) => Err(e),
    }
}

#[allow(dead_code)]
pub async fn get_by_torrent_name(
    query_progress_id: &String,
    query_torrent_name: &String,
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
) -> Result<AnimeProgress, diesel::result::Error> {
    match anime_progress
        .filter(progress_id.eq(&query_progress_id))
        .filter(torrent_name.eq(&query_torrent_name))
        .first::<AnimeProgress>(db_connection)
    {
        Ok(result) => Ok(result),
        Err(e) => Err(e),
    }
}

#[allow(dead_code)]
pub async fn delete_by_progress_id(
    query_progress_id: &String,
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
) -> Result<(), diesel::result::Error> {
    delete(anime_progress.filter(progress_id.eq(&query_progress_id)))
        .execute(db_connection)
        .expect("Error deleting anime_task");
    Ok(())
}

#[allow(dead_code)]
pub async fn delete_by_mikan_id_and_episode(
    query_progress_id: &String,
    query_mikan_id: &i32,
    query_episode: &i32,
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
) -> Result<(), diesel::result::Error> {
    delete(
        anime_progress
            .filter(progress_id.eq(&query_progress_id))
            .filter(mikan_id.eq(&query_mikan_id))
            .filter(episode.eq(&query_episode)),
    )
    .execute(db_connection)
    .expect("Error deleting anime_task");
    Ok(())
}

#[allow(dead_code)]
pub async fn delete_by_torrent_name(
    query_progress_id: &String,
    query_torrent_name: &String,
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
) -> Result<(), diesel::result::Error> {
    delete(
        anime_progress
            .filter(progress_id.eq(&query_progress_id))
            .filter(torrent_name.eq(&query_torrent_name)),
    )
    .execute(db_connection)
    .expect("Error deleting anime_task");
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

        let test_anime_progress = AnimeProgressJson {
            progress_id: "test".to_string(),
            mikan_id: 114514,
            episode: 1919810,
            torrent_name: "test_torrent_name".to_string(),
            progress_status: 110,
        };

        add_with_torrent_name(&test_anime_progress, db_connection).await.unwrap();
        let r = get_by_mikan_id_and_episode(&"test".to_string(), &114514, &1919810, db_connection)
            .await
            .unwrap();

        print!("{:?}", r);
    }
}
