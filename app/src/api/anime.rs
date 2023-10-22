use crate::{Pool, models::anime_list::AnimeListJson, dao::anime_list::*};
use actix_web::{post, get, delete, web, HttpResponse, Error};
use anyhow::Result;
use tera::Context;

#[get("/")]
pub async fn index(tera: web::Data<tera::Tera>) -> Result<HttpResponse, Error> {
    let mut context = Context::new();
    context.insert("name", "John Doe");

    let rendered = tera.render("index.html", &context).expect("Failed to render template");
    Ok(HttpResponse::Ok().content_type("text/html").body(rendered))
}

#[post("/add_anime_list")]
pub async fn add_anime_list(
        pool: web::Data<Pool>,
        item: web::Json<AnimeListJson>
    ) -> Result<HttpResponse, Error> {
        Ok(
            match add_single_anime_list(pool, item)
                .await {
                    Ok(anime_list) => HttpResponse::Created().json(anime_list),
                    _ => HttpResponse::from(HttpResponse::InternalServerError()),
                },
        )
} 

#[get("/get_all_anime_list")]
pub async fn get_all_anime_list(
    pool: web::Data<Pool>
) -> Result<HttpResponse, Error> {
    Ok(
        match get_all(pool)
        .await {
            Ok(anime_list) => HttpResponse::Created().json(anime_list),
            _ => HttpResponse::from(HttpResponse::InternalServerError()),
        },
    )
}

#[delete("/delete_anime_list/{id}")]
pub async fn del_anime_list(
    pool: web::Data<Pool>,
    path: web::Path<String>
) -> Result<HttpResponse, Error> {
    Ok(
        match del(pool, path)
        .await {
            Ok(anime_list) => HttpResponse::Created().json(anime_list),
            _ => HttpResponse::from(HttpResponse::InternalServerError()),
        },
    )
}