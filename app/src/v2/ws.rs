use crate::mods::web_socket::WebSocketActor;
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use anyhow::Result;

pub async fn ws_index(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    log::info!("ws_index: /v2/ws/");
    ws::start(WebSocketActor::new().await, &req, stream)
}
