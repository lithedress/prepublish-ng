use std::sync::Arc;
use std::{net::SocketAddr, str::FromStr};

use mongodm::prelude::MongoClient;

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
    let mongo_db = MongoClient::with_uri_str(config.mongo_srv_url)
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
    let app = routes::new().with_state(state::AppState {
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

#[cfg(test)]
mod tests {}
