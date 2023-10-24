use anyhow::Result;
use diesel::RunQueryDsl;
use diesel::dsl::insert_into;
use diesel::prelude::*;
use diesel::r2d2::{PooledConnection, ConnectionManager};
use crate::models::anime_subgroup::*;
use crate::schema::anime_subgroup::dsl::*;

// insert single data into anime_subgroup
#[allow(dead_code)]
pub async fn add(
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
    item: AnimeSubgroupJson
) -> Result<AnimeSubgroup, diesel::result::Error> {
    match anime_subgroup.filter(subgroup_id.eq(&item.subgroup_id)).first::<AnimeSubgroup>(db_connection) {
        Ok(result) => Ok(result),
        Err(_) => {
            let new_anime_subgroup = PostAnimeSubgroup{
                subgroup_id        : &item.subgroup_id,
                subgroup_name      : &item.subgroup_name,
            };
            insert_into(anime_subgroup)
                .values(&new_anime_subgroup)
                .execute(db_connection)
                .expect("Error saving new anime");
            let result = anime_subgroup.order(id.desc())
                .first(db_connection).unwrap(); 
            Ok(result)
        }
    }
}

// insert Vec<data> into anime_subgroup
pub async fn add_vec(
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
    item_vec: Vec<AnimeSubgroupJson>
) -> Result<i32, diesel::result::Error> {
    let mut sucess_num: i32 = 0;
    for item in &item_vec {
        if let Err(_) = anime_subgroup.filter(subgroup_id.eq(&item.subgroup_id)).first::<AnimeSubgroup>(db_connection) {
            let new_anime_subgroup = PostAnimeSubgroup{
                subgroup_id   : &item.subgroup_id,
                subgroup_name : &item.subgroup_name,
            };
            insert_into(anime_subgroup)
                .values(&new_anime_subgroup)
                .execute(db_connection)
                .expect("save failed");
            sucess_num += 1;
        }
    }
    Ok(sucess_num)
}

// get data by subgroup_id
pub async fn get_by_subgroupid(
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
    query_subgroupid: i32
) -> Result<AnimeSubgroup, diesel::result::Error> {
    let result: AnimeSubgroup = anime_subgroup
        .filter(subgroup_id.eq(query_subgroupid))
        .first::<AnimeSubgroup>(db_connection)?;
    Ok(result)
}

// query all data from anime_list
pub async fn get_all(
    db_connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
) -> Result<Vec<AnimeSubgroup>, diesel::result::Error> {
    let result: Vec<AnimeSubgroup> = anime_subgroup.load::<AnimeSubgroup>(db_connection)?;
    Ok(result)
}