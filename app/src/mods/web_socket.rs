use crate::mods::{config::Config, qb_api::QbitTaskExecutor};
use actix::fut::wrap_future;
use actix::ActorContext;
use actix::Handler;
use actix::Message;
use actix::SpawnHandle;
use actix::{Actor, AsyncContext, StreamHandler};
use actix_web_actors::ws::ProtocolError;
use actix_web_actors::ws::{self, Message as WSMessage, WebsocketContext};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::from_str;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

#[derive(Message, Serialize, Deserialize, Debug)]
#[rtype(result = "()")]
pub struct TextMessage(pub String);

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct GetSeedsProgressRequest {
    pub id: i32,
    pub mikan_id: i32,
    pub episode: i32,
    pub torrent_name: String,
    pub qb_task_status: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SeedProgressReply<'a> {
    pub task_type: &'a str,
    pub task_data: Vec<QbTaskJson>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct StartGetVideoProgressRequest {
    pub torrent_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct StartGetVideoProgressReply<'a> {
    pub task_type: &'a str,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct GetVideoProgressReply {
    pub torrent_name: String,
    pub progress: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(tag = "task_type")]
enum ClientRequest {
    GetSeedsProgressRequest {
        task_data: Vec<GetSeedsProgressRequest>,
    },
    StartGetVideoProgressRequest {
        task_data: StartGetVideoProgressRequest,
    },
    GetVideoProgressReply {
        task_data: GetVideoProgressReply,
    },
    StopGetSeedsProgressRequest,
    StopGetVideoProgressRequest,
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QbTaskJson {
    pub torrent_name: String,
    pub progress: String,
}

pub struct WebSocketActor {
    pub qb: QbitTaskExecutor,
    pub task_map: Arc<RwLock<HashMap<&'static str, SpawnHandle>>>,
}

impl Handler<TextMessage> for WebSocketActor {
    type Result = ();

    fn handle(&mut self, msg: TextMessage, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}

impl Actor for WebSocketActor {
    type Context = WebsocketContext<Self>;
}

impl StreamHandler<Result<WSMessage, ws::ProtocolError>> for WebSocketActor {
    fn handle(&mut self, msg: Result<WSMessage, ProtocolError>, ctx: &mut Self::Context) {
        if let Ok(WSMessage::Text(text)) = msg {
            if text == "STOP" {
                log::info!("websocket disconnect");
                ctx.stop();
                return;
            }

            match from_str::<ClientRequest>(&text) {
                Ok(clietn_request) => {
                    match clietn_request {
                        ClientRequest::GetSeedsProgressRequest { task_data } => {
                            let mut task_map_unlock = self.task_map.write().unwrap();
                            let task_handle = self.get_progress_handle(task_data, ctx);
                            task_map_unlock.insert("GetSeedsProgressRequest", task_handle);
                            drop(task_map_unlock);
                        }
                        ClientRequest::StopGetSeedsProgressRequest => {
                            let mut task_map_unlock = self.task_map.write().unwrap();
                            if task_map_unlock.contains_key("GetSeedsProgressRequest") {
                                let task_handle =
                                    task_map_unlock.get("GetSeedsProgressRequest").unwrap();
                                ctx.cancel_future(*task_handle);
                                task_map_unlock.remove_entry("GetSeedsProgressRequest");
                                drop(task_map_unlock);
                            }
                        }
                        ClientRequest::StartGetVideoProgressRequest { task_data } => {
                            let mut task_map_unlock = self.task_map.write().unwrap();
                            let task_handle =
                                self.start_get_video_progress_task_handle(task_data, ctx);
                            task_map_unlock.insert("StartGetVideoProgressRequest", task_handle);
                            drop(task_map_unlock);
                        }

                        ClientRequest::StopGetVideoProgressRequest => {
                            let mut task_map_unlock = self.task_map.write().unwrap();
                            if task_map_unlock.contains_key("StartGetVideoProgressRequest") {
                                let task_handle =
                                    task_map_unlock.get("StartGetVideoProgressRequest").unwrap();
                                ctx.cancel_future(*task_handle);
                                task_map_unlock.remove_entry("StartGetVideoProgressRequest");
                                drop(task_map_unlock);
                            }
                        }
                        ClientRequest::GetVideoProgressReply { task_data:_ } => {
                            // println!(
                            //     "task_type: {}, task_data:{:?}",
                            //     "GetVideoProgressTask", task_data
                            // );
                        }
                        _ => {}
                    };
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
        let config = Config::load_config("./config/config.yaml").await.unwrap();
        WebSocketActor {
            qb: QbitTaskExecutor::new_with_config(&config).await.unwrap(),
            task_map: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn get_progress_handle(
        &self,
        task_data: Vec<GetSeedsProgressRequest>,
        ctx: &mut <WebSocketActor as Actor>::Context,
    ) -> SpawnHandle {
        let qb: QbitTaskExecutor = self.qb.clone();
        let tasks_shared = Arc::new(RwLock::new(task_data));
        let actor_address = ctx.address().clone();

        let task_handle = ctx.run_interval(Duration::from_secs(2), move |_, ctx| {
            let qb_clone = qb.clone();
            let tasks_clone = Arc::clone(&tasks_shared);
            let actor_address_clone = actor_address.clone();

            let fut = async move {
                let qb_tasks;
                let failed_tasks;

                {
                    let task_lock = tasks_clone.read().unwrap();
                    let tasks = task_lock.clone();
                    drop(task_lock);

                    let (qt, ft) = get_qb_task(&qb_clone, tasks).await;
                    qb_tasks = qt;
                    failed_tasks = ft;
                }

                {
                    let mut tasks = tasks_clone.write().unwrap();
                    tasks.retain(|task| !failed_tasks.contains(task));
                }

                if !qb_tasks.is_empty() {
                    let json_str = serde_json::to_string(&SeedProgressReply {
                        task_type: "SeedProgressReply",
                        task_data: qb_tasks,
                    })
                    .unwrap_or_else(|_| "[]".to_string());
                    actor_address_clone.do_send(TextMessage(json_str));
                }
            };

            ctx.spawn(wrap_future(fut));
        });
        task_handle
    }

    fn start_get_video_progress_task_handle(
        &self,
        task_data: StartGetVideoProgressRequest,
        ctx: &mut <WebSocketActor as Actor>::Context,
    ) -> SpawnHandle {
        let tasks_shared = Arc::new(RwLock::new(task_data));
        let actor_address = ctx.address().clone();

        let task_handle = ctx.run_interval(Duration::from_secs(2), move |_, ctx| {
            let _tasks_clone = Arc::clone(&tasks_shared);
            let actor_address_clone = actor_address.clone();

            let fut = async move {
                let json_str = serde_json::to_string(&StartGetVideoProgressReply {
                    task_type: "GetVideoProgressRequest",
                })
                .unwrap_or_else(|_| "[]".to_string());
                actor_address_clone.do_send(TextMessage(json_str));
            };

            ctx.spawn(wrap_future(fut));
        });
        task_handle
    }
}

pub async fn get_qb_task(
    qb: &QbitTaskExecutor,
    task_list: Vec<GetSeedsProgressRequest>,
) -> (Vec<QbTaskJson>, Vec<GetSeedsProgressRequest>) {
    let mut task_qb_info_list: Vec<QbTaskJson> = Vec::new();
    let mut failed_tasks: Vec<GetSeedsProgressRequest> = Vec::new();

    for t in task_list {
        match qb.qb_api_torrent_info(&t.torrent_name).await {
            Ok(torrent_info) => {
                task_qb_info_list.push(QbTaskJson {
                    torrent_name: t.torrent_name,
                    progress: torrent_info.done,
                });
            }
            Err(_) => {
                failed_tasks.push(t);
            }
        }
    }
    (task_qb_info_list, failed_tasks)
}
