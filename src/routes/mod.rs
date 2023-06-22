mod account;
mod comment;
mod common;
mod review;
mod thesis;
mod version;
mod magazine;
mod file;

use crate::state::AppState;
use axum::{Router, routing};
use axum_csrf_sync_pattern::CsrfLayer;
use axum_sessions::async_session::MemoryStore;
use axum_sessions::SessionLayer;
use rand::Rng;
use tower_http::cors::{Any, CorsLayer};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(components(schemas(crate::mongo_entities::ObjectIdDef)))]
struct ApiDoc;

pub(crate) fn new() -> Router<AppState> {
    account::new()
        .nest("/magazines", magazine::new())
        .nest("/theses", thesis::new())
        .nest("/versions", version::new())
        .nest("/reviews", review::new())
        .nest("/comments", comment::new())
        .route("/", routing::get(|| async {}))
        .layer(CsrfLayer::new())
        .layer(SessionLayer::new(
            MemoryStore::new(),
            &rand::thread_rng().gen::<[u8; 128]>(),
        ))
        .layer(CorsLayer::new()
            .allow_origin(Any)
            .allow_headers(Any)
            .expose_headers(["X-CSRF-TOKEN".parse().unwrap()]))
}
