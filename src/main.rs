use std::{net::SocketAddr, str::FromStr, sync::Arc};

use axum::http::{header, HeaderValue, Method};

mod cfg;
mod mongo_entities;
mod routes;
mod sql_entities;
mod state;

#[tokio::main]
async fn main() {
    let config = tokio::task::spawn_blocking(cfg::AppConfig::new)
        .await
        .unwrap();
    let sql_db = sea_orm::Database::connect(config.sql_db_url).await.unwrap();
    let mongo_db = mongodm::prelude::MongoClient::with_uri_str(config.mongo_srv_url)
        .await
        .unwrap()
        .database(&config.mongo_db_nm);
    let hash_cost = config.hash_cost;
    let sender = Arc::new(config.sender);
    let smtp = <lettre::AsyncSmtpTransport<lettre::Tokio1Executor>>::relay(&config.relay)
        .unwrap()
        .port(465)
        .credentials(lettre::transport::smtp::authentication::Credentials::new(
            config.smtp_username,
            config.smtp_password,
        ))
        .build::<lettre::Tokio1Executor>();
    //assert!(smtp.test_connection().await.unwrap());
    let app = routes::new()
        .layer(
            tower_http::cors::CorsLayer::new()
                .allow_origin(config.clt_addr.parse::<HeaderValue>().unwrap())
                .allow_headers([
                    header::ACCEPT,
                    header::CONTENT_TYPE,
                    header::CONTENT_DISPOSITION,
                    header::CONTENT_ENCODING,
                    header::CONTENT_LENGTH,
                    header::COOKIE,
                    header::SET_COOKIE,
                    header::HeaderName::from_str("x-csrf-token").unwrap(),
                ])
                .allow_methods([
                    Method::HEAD,
                    Method::GET,
                    Method::POST,
                    Method::PATCH,
                    Method::DELETE,
                ])
                .allow_credentials(true)
                .expose_headers(["x-csrf-token".parse().unwrap()]),
        )
        .with_state(state::AppState {
            sql_db,
            mongo_db,
            hash_cost,
            sender,
            smtp,
        });
    axum::Server::bind(&SocketAddr::from_str(&config.srv_addr).unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
