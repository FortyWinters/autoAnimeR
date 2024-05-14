use crate::mods::qb_api::QbitTaskExecutor;
use actix::fut::wrap_future;
use actix::ActorContext;
use actix::Handler;
use actix::Message;
use actix::{Actor, AsyncContext, StreamHandler};
use actix_web_actors::ws::ProtocolError;
use actix_web_actors::ws::{self, Message as WSMessage, WebsocketContext};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::from_str;
use std::sync::Arc;
use std::time::Duration;

#[derive(Message, Serialize, Deserialize, Debug)]
#[rtype(result = "()")]
pub struct TextMessage(pub String);

impl Handler<TextMessage> for WebSocketActor {
    type Result = ();

    fn handle(&mut self, msg: TextMessage, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}

pub struct WebSocketActor {
    pub qb: QbitTaskExecutor,
}

impl Actor for WebSocketActor {
    type Context = WebsocketContext<Self>;
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TaskRequestJson {
    pub id: i32,
    pub mikan_id: i32,
    pub episode: i32,
    pub torrent_name: String,
    pub qb_task_status: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QbTaskJson {
    pub torrent_name: String,
    pub progress: String,
}

impl StreamHandler<Result<WSMessage, ws::ProtocolError>> for WebSocketActor {
    fn handle(&mut self, msg: Result<WSMessage, ProtocolError>, ctx: &mut Self::Context) {
        if let Ok(WSMessage::Text(text)) = msg {
            if text == "STOP" {
                log::info!("websocket disconnect");
                ctx.stop();
                return;
            }

            match from_str::<Vec<TaskRequestJson>>(&text) {
                Ok(tasks) => {
                    let qb = self.qb.clone();
                    let tasks_shared = Arc::new(tasks);
                    let actor_address = ctx.address().clone();

                    ctx.run_interval(Duration::from_secs(2), move |_, ctx| {
                        let qb_clone = qb.clone();
                        let tasks_clone = tasks_shared.clone();
                        let actor_address_clone = actor_address.clone();

                        let fut = async move {
                            match get_qb_task(&qb_clone, (*tasks_clone).clone()).await {
                                Ok(qb_tasks) => {
                                    let json_str = serde_json::to_string(&qb_tasks)
                                        .unwrap_or_else(|_| "[]".to_string());
                                    actor_address_clone.do_send(TextMessage(json_str));
                                }
                                Err(e) => log::info!("Error fetching qb tasks: {:?}", e),
                            }
                        };
                        ctx.spawn(wrap_future(fut));
                    });
                }
                Err(e) => {
                    log::info!("failed to parse JSON: {:?}", e);
                    ctx.text("Error parsing tasks");
                }
            }
        }
    }
}

impl WebSocketActor {
    pub async fn new() -> Self {
        WebSocketActor {
            qb: QbitTaskExecutor::new_with_login("admin".to_string(), "adminadmin".to_string())
                .await
                .unwrap(),
        }
    }
}

pub async fn get_qb_task(
    qb: &QbitTaskExecutor,
    task_list: Vec<TaskRequestJson>,
) -> Result<Vec<QbTaskJson>, anyhow::Error> {
    let mut task_qb_info_list: Vec<QbTaskJson> = Vec::new();

    for t in task_list {
        let torrent_info = qb.qb_api_torrent_info(&t.torrent_name).await.unwrap();

        task_qb_info_list.push(QbTaskJson {
            torrent_name: t.torrent_name,
            progress: torrent_info.done,
        });
    }
    Ok(task_qb_info_list)
}
