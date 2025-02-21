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
                let result = $handler(web_data).await.map_err(|e| {
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
                let result = $handler(
                    web_data,
                    item
                ).await.map_err(|e| {
                    log::error!("{} failed: {:?}", stringify!($handler), e);
                    actix_web::error::ErrorInternalServerError("Internal server error")
                })?;
                Ok(HttpResponse::Ok().json(result))
            }
        }
    };
}
