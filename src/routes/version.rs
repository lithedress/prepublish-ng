use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::{debug_handler, routing, Json, Router};
use mongodm::bson::to_document;
use mongodm::prelude::{to_bson, MongoFindOneAndUpdateOptions, MongoReturnDocument, ObjectId};
use mongodm::{
    doc, field,
    prelude::{Pull, Set},
    ToRepository,
};

use crate::mongo_entities::thesis::{
    Comment, CommentTargetType, Review, ReviewPattern, ReviewState, Version, VersionState,
};
use crate::routes::common::auth::{AuthInfo, Permission};
use crate::routes::common::err::AppError;
use crate::state::AppState;

pub(super) async fn find_version_by_id(
    state: &AppState,
    id: ObjectId,
) -> Result<Version, AppError> {
    let res = state
        .mongo_db
        .repository::<Version>()
        .find_one(
            doc! {
                "_id": id
            },
            None,
        )
        .await?
        .ok_or(AppError::NotFound(format!(
            "Version with id {} does not exist!",
            id
        )))?;
    Ok(res)
}

#[debug_handler]
async fn get(
    auth_info: AuthInfo,
    State(state): State<AppState>,
    Path(id): Path<ObjectId>,
) -> Result<Json<Version>, AppError> {
    let res = find_version_by_id(&state, id).await?;
    let thesis = super::thesis::find_thesis_by_id(&state, res.thesis_id).await?;
    if !(auth_info.permitted(Permission::Publishing)
        || res.uploader_id == Some(auth_info.id()?)
        || thesis.id.owner_id == auth_info.id()?
        || thesis.author_ids.contains(&auth_info.id()?))
    {
        match &res {
            Version {
                state: VersionState::History,
                ..
            }
            | Version {
                state: VersionState::Passed(true),
                ..
            } => {}
            Version {
                state: VersionState::Reviewing,
                review_state:
                    ReviewState {
                        remainder_reviewer_ids,
                        ..
                    },
                ..
            } => {
                if !remainder_reviewer_ids.contains(&auth_info.id()?) {
                    return Err(AppError::Forbidden(format!(
                        "You are not allowed to review version {}!",
                        id
                    )));
                }
            }
            _ => {
                return Err(AppError::Forbidden(format!(
                    "Version {} is not public!",
                    id
                )));
            }
        }
    }
    Ok(Json(res))
}

// #[debug_handler]
// async fn download(
//     auth_info: AuthInfo,
//     State(state): State<AppState>,
//     Path(id): Path<ObjectId>,
// ) -> Result<
//     (
//         TypedHeader<ContentDisposition>,
//         TypedHeader<ContentLength>,
//         TypedHeader<ContentType>,
//         StreamBody<impl Stream<Item = std::io::Result<Vec<u8>>> + Sized>,
//     ),
//     AppError,
// > {
//     let db = state.mongo_db.clone();
//     let Json(res) = get(auth_info, State(state), Path(id)).await?;
//     let bucket = db.gridfs_bucket(None);
//
//     let doc = bucket
//         .find(
//             doc! {
//                 "_id": res.file_id
//             },
//             None,
//         )
//         .await?
//         .next()
//         .await
//         .ok_or(anyhow::anyhow!("File {} lost!", res.file_id))??;
//     let content_disposition =
//         ContentDisposition::decode(&mut std::iter::once(&HeaderValue::try_from(format!(
//             "{}{}{}",
//             common::DISPOSITION_PREFIX,
//             doc.filename.unwrap_or_default(),
//             common::DISPOSITION_SUFFIX
//         ))?))?;
//     let content_length = ContentLength(doc.chunk_size_bytes.into());
//     let content_type = ContentType::from(
//         Mime::from_str(
//             &doc.metadata
//                 .and_then(|md| md.get("Content-Type").map(ToString::to_string))
//                 .unwrap_or_default(),
//         )
//         .unwrap_or(mime::TEXT_PLAIN),
//     );
//
//     let stream = bucket.open_download_stream(bson!(res.file_id)).await?;
//     let stream = FramedRead::new(stream, BytesCodec).map_ok(|b| b.to_vec());
//
//     db.repository::<Version>()
//         .find_one_and_update(
//             doc! {
//                 "_id": id
//             },
//             doc! {
//                 Inc: {
//                     field!(downloads in Version): 1
//                 }
//             },
//             None,
//         )
//         .await?;
//
//     Ok((
//         TypedHeader(content_disposition),
//         TypedHeader(content_length),
//         TypedHeader(content_type),
//         StreamBody::new(stream),
//     ))
// }

#[debug_handler]
async fn edit(
    auth_info: AuthInfo,
    State(state): State<AppState>,
    Path(id): Path<ObjectId>,
    Json(mut body): Json<ReviewState>,
) -> Result<Json<Version>, AppError> {
    if !auth_info.permitted(Permission::Publishing) {
        return Err(AppError::Forbidden("you are not a editor".to_string()));
    }
    let version = find_version_by_id(&state, id).await?;
    match version.state {
        VersionState::Uploaded => {}
        _ => {
            return Err(AppError::BadRequest(format!(
                "Version {} has been edited!",
                id
            )));
        }
    }

    if let ReviewPattern::Editor(_) = body.pattern {
        body.pattern = ReviewPattern::Editor(auth_info.id()?)
    }
    let res = state
        .mongo_db
        .repository::<Version>()
        .find_one_and_update(
            doc! {
                "_id": id
            },
            doc! {
                Set: {
                    field!(state in Version): to_bson(&VersionState::Reviewing)?,
                    field!(review_state in Version): to_document(&body)?
                }
            },
            MongoFindOneAndUpdateOptions::builder()
                .return_document(MongoReturnDocument::After)
                .build(),
        )
        .await?
        .ok_or(anyhow::anyhow!("Cannot get updated id!"))?;
    Ok(Json(res))
}

#[debug_handler]
async fn review(
    auth_info: AuthInfo,
    State(state): State<AppState>,
    Path(id): Path<ObjectId>,
    Json(mut body): Json<Review>,
) -> Result<(StatusCode, Json<ObjectId>), AppError> {
    let version = find_version_by_id(&state, id).await?;
    match version.state {
        VersionState::Reviewing => {}
        VersionState::Uploaded => {
            return Err(AppError::BadRequest(format!(
                "Version {} has not been edited!",
                id
            )));
        }
        _ => {
            return Err(AppError::BadRequest(format!(
                "Version {} has been adjudged!",
                id
            )));
        }
    }
    if !version
        .review_state
        .remainder_reviewer_ids
        .contains(&auth_info.id()?)
    {
        return Err(AppError::Forbidden(format!(
            "You are not allowed to review version {}!",
            id
        )));
    }

    body._id = ObjectId::new();
    body.version_id = version._id;
    body.reviewer_id = Some(auth_info.id()?);
    body.reviewed_at = chrono::Utc::now();
    let res = state
        .mongo_db
        .repository::<Review>()
        .insert_one(body, None)
        .await?
        .inserted_id
        .as_object_id()
        .ok_or(anyhow::anyhow!("Cannot get inserted id!"))?;

    let version = state.mongo_db.repository::<Version>()
        .find_one_and_update(
            doc! {
                "_id": id
            },
            doc! {
                Pull: {
                    field!((review_state in Version).(remainder_reviewer_ids in ReviewState)): auth_info.id
                }
            },
            MongoFindOneAndUpdateOptions::builder().return_document(MongoReturnDocument::After).build()
        )
        .await?
        .ok_or(anyhow::anyhow!("Cannot get updated id!"))?;
    match &version.review_state {
        ReviewState {
            remainder_reviewer_ids,
            pattern: ReviewPattern::Reviewer,
        } if remainder_reviewer_ids.is_empty() && {
            let mut pass = true;
            let mut reviews = version.reviews(state.mongo_db.clone()).await?;
            while reviews.advance().await? {
                let review = reviews.deserialize_current()?;
                if !review.judgement {
                    pass = false;
                    break;
                }
            }
            pass
        } =>
        {
            version.pass(state.mongo_db).await
        }
        _ => version.reject(state.mongo_db).await,
    }?
    .ok_or(anyhow::anyhow!("Cannot get updated id!"))?;

    Ok((StatusCode::CREATED, Json(res)))
}

#[debug_handler]
async fn adjudge(
    auth_info: AuthInfo,
    State(state): State<AppState>,
    Path((id, judgement)): Path<(ObjectId, bool)>,
) -> Result<Json<Version>, AppError> {
    if !auth_info.permitted(Permission::Publishing) {
        return Err(AppError::Forbidden("you are not a editor".to_string()));
    }
    let version = find_version_by_id(&state, id).await?;
    match version.state {
        VersionState::Uploaded | VersionState::Reviewing => {}
        _ => {
            return Err(AppError::BadRequest(format!(
                "Version {} has been adjudged!",
                id
            )));
        }
    }
    let res = if judgement {
        version.pass(state.mongo_db).await
    } else {
        version.reject(state.mongo_db).await
    }?
    .ok_or(anyhow::anyhow!("Cannot get updated id!"))?;

    Ok(Json(res))
}

#[debug_handler]
async fn comment(
    auth_info: AuthInfo,
    State(state): State<AppState>,
    Path(id): Path<ObjectId>,
    Json(mut body): Json<Comment>,
) -> Result<(StatusCode, Json<ObjectId>), AppError> {
    let version = find_version_by_id(&state, id).await?;
    match version.state {
        VersionState::History => {
            return Err(AppError::Forbidden(format!("Version {} is outdated!", id)));
        }
        VersionState::Passed(true) => {}
        _ => {
            return Err(AppError::Forbidden(format!(
                "Version {} is not public!",
                id
            )));
        }
    }

    body.poster_id = Some(auth_info.id()?);
    body.posted_at = chrono::Utc::now();
    body.target_id = id;
    body.target_type = CommentTargetType::Version;
    let res = super::comment::insert_comment(&state, body).await?;
    Ok((StatusCode::CREATED, Json(res)))
}

pub(super) fn new() -> Router<AppState> {
    Router::new()
        .route("/:id", routing::get(get))
        //.route("/:id/file", routing::get(download))
        .route("/:id/edit", routing::patch(edit))
        .route("/:id/review", routing::post(review))
        .route("/:id/adjudge/:judgement", routing::patch(adjudge))
        .route("/:id/comment", routing::post(comment))
}
