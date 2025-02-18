#[macro_export]
macro_rules! api_handler {
    ($path:expr, $func_name:ident, "db") => {
        api_db!($path, $func_name);
    };
    ($path:expr, $func_name:ident, "db", $json_type:ty) => {
        api_db!($path, $func_name, $json_type);
    };
    ($path:expr, $func_name:ident, "qb") => {
        api_qb!($path, $func_name);
    };
    ($path:expr, $func_name:ident, "qb", $json_type:ty) => {
        api_qb!($path, $func_name, $json_type);
    };
    ($path:expr, $func_name:ident, "config") => {
        api_config!($path, $func_name);
    };
    ($path:expr, $func_name:ident, "config", $json_type:ty) => {
        api_config!($path, $func_name, $json_type);
    };
}

#[macro_export]
macro_rules! api_db {
    ($path:expr, $func_name:ident) => {
        paste::paste! {
            #[actix_web::get($path)]
            pub async fn [<$func_name _handler>](
                pool: web::Data<Pool>
            ) -> Result<HttpResponse, actix_web::Error> {
                log::info!("{}", stringify!([<$func_name _handler>]));
                let db = &mut pool
                    .get()
                    .map_err(|e| handle_error(e, "failed to get db connection"))?;
                let res = $func_name(db)
                    .await
                    .map_err(|e| {
                        log::error!("{} failed: {:?}", stringify!($func_name), e);
                        actix_web::error::ErrorInternalServerError("Internal server error")
                    })?;
                Ok(HttpResponse::Ok().json(res))
            }
        }
    };

    ($path:expr, $func_name:ident, $json_type:ty) => {
        paste::paste! {
            #[actix_web::post($path)]
            pub async fn [<$func_name _handler>](
                pool: web::Data<Pool>,
                item: web::Json<$json_type>,
            ) -> Result<HttpResponse, actix_web::Error> {
                log::info!("{}, {:?}", stringify!([<$func_name _handler>]), item);
                let db = &mut pool
                    .get()
                    .map_err(|e| handle_error(e, "failed to get db connection"))?;
                let res = $func_name(db, item.into_inner())
                    .await
                    .map_err(|e| {
                        log::error!("{} failed: {:?}", stringify!($func_name), e);
                        actix_web::error::ErrorInternalServerError("Internal server error")
                    })?;
                Ok(HttpResponse::Ok().json(res))
            }
        }
    };
}

#[macro_export]
macro_rules! api_qb {
    ($path:expr, $func_name:ident) => {
        paste::paste! {
            #[actix_web::get($path)]
            pub async fn [<$func_name _handler>](
                pool: web::Data<Pool>,
                qb: QB
            ) -> Result<HttpResponse, actix_web::Error> {
                log::info!("{}", stringify!([<$func_name _handler>]));
                let db = &mut pool
                    .get()
                    .map_err(|e| handle_error(e, "failed to get db connection"))?;
                let res = $func_name(db, qb)
                    .await
                    .map_err(|e| {
                        log::error!("{} failed: {:?}", stringify!($func_name), e);
                        actix_web::error::ErrorInternalServerError("Internal server error")
                    })?;
                Ok(HttpResponse::Ok().json(res))
            }
        }
    };

    ($path:expr, $func_name:ident, $json_type:ty) => {
        paste::paste! {
            #[actix_web::post($path)]
            pub async fn [<$func_name _handler>](
                pool: web::Data<Pool>,
                qb: QB,
                item: web::Json<$json_type>,
            ) -> Result<HttpResponse, actix_web::Error> {
                log::info!("{}, {:?}", stringify!([<$func_name _handler>]), item);
                let db = &mut pool
                    .get()
                    .map_err(|e| handle_error(e, "failed to get db connection"))?;
                let res = $func_name(db, qb, item.into_inner())
                    .await
                    .map_err(|e| {
                        log::error!("{} failed: {:?}", stringify!($func_name), e);
                        actix_web::error::ErrorInternalServerError("Internal server error")
                    })?;
                Ok(HttpResponse::Ok().json(res))
            }
        }
    };
}

#[macro_export]
macro_rules! api_config {
    ($path:expr, $func_name:ident) => {
        paste::paste! {
            #[actix_web::get($path)]
            pub async fn [<$func_name _handler>](
                pool: web::Data<Pool>,
                config: CONFIG
            ) -> Result<HttpResponse, actix_web::Error> {
                log::info!("{}", stringify!([<$func_name _handler>]));
                let db = &mut pool
                    .get()
                    .map_err(|e| handle_error(e, "failed to get db connection"))?;
                let res = $func_name(db, config)
                    .await
                    .map_err(|e| {
                        log::error!("{} failed: {:?}", stringify!($func_name), e);
                        actix_web::error::ErrorInternalServerError("Internal server error")
                    })?;
                Ok(HttpResponse::Ok().json(res))
            }
        }
    };

    ($path:expr, $func_name:ident, $json_type:ty) => {
        paste::paste! {
            #[actix_web::post($path)]
            pub async fn [<$func_name _handler>](
                pool: web::Data<Pool>,
                config: CONFIG,
                item: web::Json<$json_type>,
            ) -> Result<HttpResponse, actix_web::Error> {
                log::info!("{}, {:?}", stringify!([<$func_name _handler>]), item);
                let db = &mut pool
                    .get()
                    .map_err(|e| handle_error(e, "failed to get db connection"))?;
                let res = $func_name(db, config, item.into_inner())
                    .await
                    .map_err(|e| {
                        log::error!("{} failed: {:?}", stringify!($func_name), e);
                        actix_web::error::ErrorInternalServerError("Internal server error")
                    })?;
                Ok(HttpResponse::Ok().json(res))
            }
        }
    };
}

pub fn handle_error<E: std::fmt::Debug>(e: E, message: &str) -> actix_web::Error {
    log::error!("{}, error: {:?}", message, e);
    actix_web::error::ErrorInternalServerError("Internal server error")
}

#[macro_export]
macro_rules! register_handler {
    (GET $path:expr => $handler:ident) => {
        paste::paste! {
            #[actix_web::get($path)]
            pub async fn [<$handler _handler>](
                web_data: web::Data<WebData>
            ) -> Result<HttpResponse, actix_web::Error> {
                log::info!("{}", stringify!([<$handler _handler>]));
                let db = &mut web_data.pool.get().map_err(|e| handle_error(e, "failed to get db connection"))?;
                let result = $handler(
                    db,
                    web_data.qb.clone(),
                    web_data.task_status.clone(),
                    web_data.video_file_lock.clone(),
                    web_data.config.clone()
                ).await.map_err(|e| {
                    log::error!("{} failed: {:?}", stringify!($handler), e);
                    actix_web::error::ErrorInternalServerError("Internal server error")
                })?;
                Ok(HttpResponse::Ok().json(result))
            }
        }
    };

    (POST $path:expr => $handler:ident, $json_type:ty) => {
        paste::paste! {
            #[actix_web::post($path)]
            pub async fn [<$handler _handler>](
                web_data: web::Data<WebData>,
                item: web::Json<$json_type>,
            ) -> Result<HttpResponse, actix_web::Error> {
                log::info!("{}, {:?}", stringify!([<$handler _handler>]), item);
                let db = &mut web_data.pool.get().map_err(|e| handle_error(e, "failed to get db connection"))?;
                let result = $handler(
                    db,
                    web_data.qb.clone(),
                    web_data.config.clone(),
                    web_data.status.clone(),
                    item.into_inner(),
                ).await.map_err(|e| {
                    log::error!("{} failed: {:?}", stringify!($handler), e);
                    actix_web::error::ErrorInternalServerError("Internal server error")
                })?;
                Ok(HttpResponse::Ok().json(result))
            }
        }
    };
}
