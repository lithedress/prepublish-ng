use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::{debug_handler, Json, Router, routing};
use mongodm::prelude::{MongoFindOneAndReplaceOptions, MongoReturnDocument, ObjectId};
use mongodm::{doc, ToRepository};
use crate::mongo_entities::paper_collection::Magazine;
use crate::routes::common::auth::{AuthInfo, Permission};
use crate::routes::common::err::AppError;
use crate::state::AppState;

#[debug_handler]
async fn post(auth_info: AuthInfo, State(state): State<AppState>, Json(mut body): Json<Magazine>) -> Result<(StatusCode, Json<ObjectId>), AppError> {
    if !auth_info.permitted(Permission::Managing) {
        return Err(AppError::Forbidden("You are not a administrator!".to_string()))
    }

    body.meta._id = ObjectId::new();
    body.modified_at = chrono::Utc::now();
    let res = state.mongo_db.repository::<Magazine>()
        .insert_one(body, None)
        .await?
        .inserted_id
        .as_object_id()
        .ok_or(anyhow::anyhow!("Cannot get inserted id!"))?;
    Ok((StatusCode::CREATED, Json(res)))
}

#[debug_handler]
async fn get(_auth_info: AuthInfo, State(state): State<AppState>, Path(id): Path<ObjectId>) -> Result<Json<Magazine>, AppError> {
    let res = state.mongo_db.repository::<Magazine>()
        .find_one(doc! {
            "_id": id
        }, None)
        .await?
        .ok_or(AppError::NotFound(format!(
            "Magazine with id {} does not exist!",
            id
        )))?;
    Ok(Json(res))
}

#[debug_handler]
async fn put(auth_info: AuthInfo, State(state): State<AppState>, Path(id): Path<ObjectId>, Json(mut body): Json<Magazine>) -> Result<Json<Magazine>, AppError> {
    if !auth_info.permitted(Permission::Managing) {
        return Err(AppError::Forbidden("You are not a administrator!".to_string()))
    }

    body.meta._id = id;
    body.modified_at = chrono::Utc::now();
    let res = state.mongo_db.repository::<Magazine>()
        .find_one_and_replace(doc! {
            "_id": id
        }, body, MongoFindOneAndReplaceOptions::builder().return_document(MongoReturnDocument::After).build())
        .await?
        .ok_or(anyhow::anyhow!("Cannot get updated thesis!"))?;
    Ok(Json(res))
}

pub(super) fn new() -> Router<AppState> {
    Router::new()
        .route("/", routing::post(post))
        .route("/:id", routing::get(get).put(put))
}