use crate::mongo_entities::thesis::{Comment, CommentTargetType};
use crate::routes::common::auth::AuthInfo;
use crate::routes::common::err::AppError;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::{debug_handler, routing, Json, Router};
use mongodm::prelude::ObjectId;
use mongodm::{doc, ToRepository};

#[debug_handler]
async fn get(
    _auth_info: AuthInfo,
    State(state): State<AppState>,
    Path(id): Path<ObjectId>,
) -> Result<Json<Comment>, AppError> {
    let res = find_comment_by_id(&state, id).await?;
    Ok(Json(res))
}

async fn find_comment_by_id(state: &AppState, id: ObjectId) -> Result<Comment, AppError> {
    state
        .mongo_db
        .repository::<Comment>()
        .find_one(
            doc! {
                "_id": id
            },
            None,
        )
        .await?
        .ok_or(AppError::NotFound(format!(
            "Comment with id {} does not exist!",
            id
        )))
}

#[debug_handler]
async fn reply(
    auth_info: AuthInfo,
    State(state): State<AppState>,
    Path(id): Path<ObjectId>,
    Json(mut body): Json<Comment>,
) -> Result<(StatusCode, Json<ObjectId>), AppError> {
    find_comment_by_id(&state, id).await?;

    body.poster_id = Some(auth_info.id);
    body.posted_at = chrono::Utc::now();
    body.target_type = CommentTargetType::Comment;
    body.target_id = id;
    let res = insert_comment(&state, body).await?;
    Ok((StatusCode::CREATED, Json(res)))
}

pub(super) async fn insert_comment(state: &AppState, body: Comment) -> Result<ObjectId, AppError> {
    state
        .mongo_db
        .repository::<Comment>()
        .insert_one(body, None)
        .await?
        .inserted_id
        .as_object_id()
        .ok_or(anyhow::anyhow!("Cannot get updated id!").into())
}

pub(super) fn new() -> Router<AppState> {
    Router::new()
        .route("/:id", routing::get(get))
        .route("/:id/reply", routing::post(reply))
}
