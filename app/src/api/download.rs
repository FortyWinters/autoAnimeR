use actix_web::{get, web, HttpResponse, Error};
use anyhow::Result;
use tera::Context;
use crate::api::anime::{BroadcastUrl, get_broadcast_map};

#[get("/")]
pub async fn download_index_handler(
    tera: web::Data<tera::Tera>
) -> Result<HttpResponse, Error> {
    Ok(
        match download_index(tera)
            .await {
                Ok(res) => res,
                _ => HttpResponse::from(HttpResponse::InternalServerError()),
            },
    )
}

pub async fn download_index(
    tera: web::Data<tera::Tera>
) -> Result<HttpResponse, Error> {
    let broadcast_url = BroadcastUrl { url_year: 0, url_season : 0 };
    let broadcast_map = get_broadcast_map().await;
    let mut context = Context::new();
    context.insert("broadcast_map", &broadcast_map);
    context.insert("broadcast_url", &broadcast_url);
    context.insert("page_flag", &0);
    let rendered = tera.render("download.html", &context).expect("Failed to render template");
    Ok(HttpResponse::Ok().content_type("text/html").body(rendered))
}