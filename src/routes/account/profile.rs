use axum::extract::{Path, Query, State};
use axum::http::{HeaderName, HeaderValue};
use axum::{debug_handler, routing, Json, Router};
use futures::TryStreamExt;
use futures_util::Stream;
use mongodm::bson::Document;
use mongodm::prelude::{
    MongoFindOneAndReplaceOptions, MongoFindOptions, MongoReturnDocument, ObjectId,
};
use mongodm::{doc, ToRepository};

use crate::mongo_entities::profile::{Profile, PublicProfile};
use crate::routes::common::auth::AuthInfo;
use crate::routes::common::err::AppError;
use crate::routes::common::query::AppQuery;
use crate::state::AppState;

#[debug_handler]
async fn index(
    auth_info: AuthInfo,
    State(state): State<AppState>,
) -> Result<Json<Profile>, AppError> {
    let id = auth_info.id()?;
    let res = try_find_profile_by_id(&state, id)
        .await?
        .ok_or(anyhow::anyhow!("Your profile {} has been lost!", id))?;
    Ok(Json(res))
}

#[debug_handler]
async fn change(
    auth_info: AuthInfo,
    State(state): State<AppState>,
    Json(mut body): Json<Profile>,
) -> Result<Json<Profile>, AppError> {
    let id = auth_info.id()?;
    let old = try_find_profile_by_id(&state, id)
        .await?
        .ok_or(anyhow::anyhow!("Your profile {} has been lost!", id))?;

    body.public_profile.id = old.public_profile.id;
    let res = state
        .mongo_db
        .repository::<Profile>()
        .find_one_and_replace(
            doc! {
                "_id": id
            },
            body,
            MongoFindOneAndReplaceOptions::builder()
                .return_document(MongoReturnDocument::After)
                .build(),
        )
        .await?
        .ok_or(anyhow::anyhow!("Cannot get inserted id!"))?;
    Ok(Json(res))
}

async fn try_find_profile_by_id(
    state: &AppState,
    id: ObjectId,
) -> Result<Option<Profile>, AppError> {
    let res = state
        .mongo_db
        .repository::<Profile>()
        .find_one(
            doc! {
                "_id": id
            },
            None,
        )
        .await?;
    Ok(res)
}

#[debug_handler]
async fn get(
    _auth_info: AuthInfo,
    State(state): State<AppState>,
    Path(id): Path<ObjectId>,
) -> Result<Json<PublicProfile>, AppError> {
    let res = try_find_profile_by_id(&state, id)
        .await?
        .ok_or(AppError::NotFound(format!(
            "Profile with id {} does not exist!",
            id
        )))?
        .public_profile;
    Ok(Json(res))
}

#[debug_handler]
async fn gets(
    _auth_info: AuthInfo,
    State(state): State<AppState>,
    Query(query): Query<AppQuery>,
    Json(body): Json<Document>,
) -> Result<([(HeaderName, HeaderValue); 4], Json<Vec<PublicProfile>>), AppError> {
    let count = state
        .mongo_db
        .repository::<Profile>()
        .count_documents(body.clone(), None)
        .await?;
    let mut cur = state
        .mongo_db
        .repository::<Profile>()
        .find(
            body,
            MongoFindOptions::builder()
                .skip(query.offset)
                .limit(query.limit)
                .build(),
        )
        .await?;
    let mut res = Vec::with_capacity(cur.size_hint().1.unwrap_or_default());
    while let Some(profile) = cur.try_next().await? {
        res.push(profile.public_profile)
    }
    Ok((query.pagenate(count), Json(res)))
}

pub(super) fn new() -> Router<AppState> {
    Router::new()
        .route("/profile", routing::get(index).put(change))
        .merge(
            Router::new()
                .route("/profiles", routing::get(gets))
                .route("/profiles/:id", routing::get(get)),
        )
}
