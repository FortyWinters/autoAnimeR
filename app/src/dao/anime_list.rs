use anyhow::Result;
use diesel::{RunQueryDsl, delete};
use diesel::dsl::insert_into;
use diesel::prelude::*;
use diesel::r2d2::{PooledConnection, ConnectionManager};
use crate::models::anime_list::*;
use crate::schema::anime_list::dsl::*;

// insert single data into anime_list
#[allow(dead_code)]
pub async fn add(
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
    item: AnimeListJson
) -> Result<AnimeList, diesel::result::Error> {
    match anime_list.filter(mikan_id.eq(&item.mikan_id)).first::<AnimeList>(db_connection) {
        Ok(result) => Ok(result),
        Err(_) => {
            let new_anime_list = PostAnimeList{
                mikan_id        : &item.mikan_id,
                anime_name      : &item.anime_name,
                img_url         : &item.img_url,
                update_day      : &item.update_day,
                anime_type      : &item.anime_type,
                subscribe_status: &item.subscribe_status
            };
            insert_into(anime_list)
                .values(&new_anime_list)
                .execute(db_connection)
                .expect("Error saving new anime");
            let result = anime_list.order(id.desc())
                .first(db_connection).unwrap(); 
            Ok(result)
        }
    }
}

// insert Vec<data> into anime_list
pub async fn add_vec(
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
    item_vec: Vec<AnimeListJson>
) -> Result<i32, diesel::result::Error> {
    let mut sucess_num: i32 = 0;
    for item in &item_vec {
        if let Err(_) = anime_list.filter(mikan_id.eq(&item.mikan_id)).first::<AnimeList>(db_connection) {
            let new_anime_list = PostAnimeList{
                mikan_id        : &item.mikan_id,
                anime_name      : &item.anime_name,
                img_url         : &item.img_url,
                update_day      : &item.update_day,
                anime_type      : &item.anime_type,
                subscribe_status: &item.subscribe_status
            };
            insert_into(anime_list)
                .values(&new_anime_list)
                .execute(db_connection)
                .expect("save failed");
            sucess_num += 1;
        }
    }
    Ok(sucess_num)
}

// get data by mikan_id
pub async fn get_by_mikanid(
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
    query_mikanid: i32
) -> Result<AnimeList, diesel::result::Error> {
    let result: AnimeList = anime_list
        .filter(mikan_id.eq(query_mikanid))
        .first::<AnimeList>(db_connection)?;
    Ok(result)
}

pub async fn get_by_subscribestatus(
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
    query_subscribestatus: i32
) -> Result<Vec<AnimeList>, diesel::result::Error> {
    let result: Vec<AnimeList> = anime_list
        .filter(subscribe_status.eq(query_subscribestatus))
        .load::<AnimeList>(db_connection)?;
    Ok(result)
}

// update subscribe_status by mikan_id
pub async fn update_subscribestatus_by_mikanid(
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
    query_mikanid: i32,
    update_subscribestatus: i32
) -> Result<(), diesel::result::Error> {
    diesel::update(anime_list
            .filter(mikan_id.eq(query_mikanid)))
        .set(subscribe_status.eq(update_subscribestatus))
        .execute(db_connection)?;
    Ok(())
} 

// query all data from anime_list
#[allow(dead_code)]
pub async fn get_all(
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
) -> Result<Vec<AnimeList>, diesel::result::Error> {
    let result: Vec<AnimeList> = anime_list.load::<AnimeList>(db_connection)?;
    Ok(result)
}

// delete single data by mikan_id
#[allow(dead_code)]
pub async fn del_by_mikan_id(
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
    i: i32, 
) -> Result<usize, diesel::result::Error> {
    let result = delete(anime_list.filter(mikan_id.eq(i))).execute(db_connection)?;
    Ok(result)
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
        let db_connection = &mut pool.get().unwrap();
        let test_anime_seed_json = AnimeListJson {
            mikan_id: 3143,
            anime_name: "米奇与达利".to_string(),
            update_day: 1,
            img_url: "/images/Bangumi/202310/69e733eb.jpg".to_string(),
            anime_type: 0,
            subscribe_status: 1
        };

        let r = add(db_connection, test_anime_seed_json).await.unwrap();
        println!("{:?}", r);
    }
}