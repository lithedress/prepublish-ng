use axum::{routing, Router};
use axum_csrf_sync_pattern::CsrfLayer;
use axum_sessions::async_session::MemoryStore;
use axum_sessions::SessionLayer;
use rand::Rng;
use utoipa::OpenApi;

use crate::state::AppState;

mod account;
mod comment;
mod common;
mod file;
mod magazine;
mod review;
mod thesis;
mod version;

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
        .nest("/files", file::new())
        .route("/", routing::get(|| async {}))
        .nest(
            "/static",
            axum_static::static_router("static").with_state(()),
        )
        //.layer(CsrfLayer::new())
        .layer(SessionLayer::new(
            MemoryStore::new(),
            &rand::thread_rng().gen::<[u8; 128]>(),
        ))
}
