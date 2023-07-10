use axum::extract::multipart::Field;
use axum::extract::{Multipart, Path, Query, State};
use axum::http::{HeaderName, HeaderValue, StatusCode};
use axum::{debug_handler, routing, Json, Router};
use futures_util::{Stream, TryStreamExt};
use mongodm::bson::Document;
use mongodm::mongo::options::GridFsUploadOptions;
use mongodm::mongo::GridFsBucket;
use mongodm::prelude::{
    MongoFindOneAndReplaceOptions, MongoFindOneOptions, MongoFindOptions, MongoReturnDocument,
    ObjectId,
};
use mongodm::{doc, field, ToRepository};

use crate::mongo_entities::paper_collection::Magazine;
use crate::mongo_entities::thesis::{ReviewState, Thesis, ThesisId, Version, VersionState};
use crate::routes::common::auth::{AuthInfo, Permission};
use crate::routes::common::err::AppError;
use crate::routes::common::query::AppQuery;
use crate::state::AppState;

#[debug_handler]
async fn post(
    auth_info: AuthInfo,
    State(state): State<AppState>,
    Json(mut body): Json<Thesis>,
) -> Result<(StatusCode, Json<ObjectId>), AppError> {
    // if state
    //     .mongo_db
    //     .repository::<Magazine>()
    //     .find_one(
    //         doc! {
    //             "_id": body.magazine_id
    //         },
    //         None,
    //     )
    //     .await?
    //     .is_none()
    // {
    //     return Err(AppError::BadRequest(format!(
    //         "No such magazine {}!",
    //         body.magazine_id
    //     )));
    // }

    body.id = ThesisId {
        _id: ObjectId::new(),
        owner_id: auth_info.id()?,
        is_passed: false,
        created_at: chrono::Utc::now(),
    };
    let res = state
        .mongo_db
        .repository::<Thesis>()
        .insert_one(body, None)
        .await?
        .inserted_id
        .as_object_id()
        .ok_or(anyhow::anyhow!("Cannot get inserted id!"))?;
    Ok((StatusCode::CREATED, Json(res)))
}

#[debug_handler]
async fn get(
    auth_info: AuthInfo,
    State(state): State<AppState>,
    Path(id): Path<ObjectId>,
) -> Result<Json<Thesis>, AppError> {
    let res = find_thesis_by_id(&state, id).await?;
    if !(auth_info.permitted(Permission::Publishing)
        || res.id.owner_id == auth_info.id()?
        || res.author_ids.contains(&auth_info.id()?))
    {
        match find_last_version(&state, id).await? {
            Some(version) if version.major_num > 0 => {}
            Some(Version {
                state: VersionState::Reviewing,
                review_state:
                    ReviewState {
                        remainder_reviewer_ids,
                        ..
                    },
                ..
            }) => {
                if !(remainder_reviewer_ids.contains(&auth_info.id()?)) {
                    return Err(AppError::Forbidden(format!(
                        "You are not allowed to review thesis {}!",
                        id
                    )));
                }
            }
            _ => {
                return Err(AppError::Forbidden(format!("Thesis {} is not public!", id)));
            }
        }
    }
    Ok(Json(res))
}

#[debug_handler]
async fn gets(
    _auth_info: AuthInfo,
    State(state): State<AppState>,
    Query(query): Query<AppQuery>,
    Json(mut body): Json<Document>,
) -> Result<([(HeaderName, HeaderValue); 4], Json<Vec<Thesis>>), AppError> {
    body.insert(field!(is_passed in ThesisId), true);
    let count = state
        .mongo_db
        .repository::<Thesis>()
        .count_documents(body.clone(), None)
        .await?;
    let res = state
        .mongo_db
        .repository::<Thesis>()
        .find(
            body,
            MongoFindOptions::builder()
                .skip(query.offset)
                .limit(query.limit)
                .build(),
        )
        .await?
        .try_collect()
        .await?;
    Ok((query.pagenate(count), Json(res)))
}

#[debug_handler]
async fn put(
    auth_info: AuthInfo,
    State(state): State<AppState>,
    Path(id): Path<ObjectId>,
    Json(mut body): Json<Thesis>,
) -> Result<Json<Thesis>, AppError> {
    let thesis = find_thesis_by_id(&state, id).await?;
    if auth_info.permitted(Permission::Publishing)
        || thesis.id.owner_id == auth_info.id()?
        || thesis.author_ids.contains(&auth_info.id()?)
    {
        return Err(AppError::Forbidden(format!(
            "You are neither an editor or an author of thesis {}!",
            id
        )));
    }

    body.id = thesis.id;
    let res = state
        .mongo_db
        .repository::<Thesis>()
        .find_one_and_replace(
            doc! {"_id": id},
            body,
            Some(
                MongoFindOneAndReplaceOptions::builder()
                    .return_document(Some(MongoReturnDocument::After))
                    .build(),
            ),
        )
        .await?
        .ok_or(anyhow::anyhow!("Cannot get updated thesis!"))?;
    Ok(Json(res))
}

pub(super) async fn find_thesis_by_id(state: &AppState, id: ObjectId) -> Result<Thesis, AppError> {
    let res = state
        .mongo_db
        .repository::<Thesis>()
        .find_one(
            doc! {
                "_id": id
            },
            None,
        )
        .await?
        .ok_or(AppError::NotFound(format!(
            "Thesis with id {} does not exist!",
            id
        )))?;
    Ok(res)
}

async fn find_last_version(state: &AppState, id: ObjectId) -> Result<Option<Version>, AppError> {
    let res = state
        .mongo_db
        .repository::<Version>()
        .find_one(
            doc! {
                field!(thesis_id in Version): id
            },
            Some(
                MongoFindOneOptions::builder()
                    .sort(doc! {
                        field!(major_num in Version): -1,
                        field!(minor_num in Version): -1
                    })
                    .build(),
            ),
        )
        .await?;
    Ok(res)
}

#[debug_handler]
async fn delete(
    auth_info: AuthInfo,
    State(state): State<AppState>,
    Path(id): Path<ObjectId>,
) -> Result<(StatusCode, Json<u64>), AppError> {
    let thesis = find_thesis_by_id(&state, id).await?;
    if !auth_info.permitted(Permission::Publishing) {
        if thesis.id.owner_id != auth_info.id()? {
            return Err(AppError::Forbidden(format!(
                "You do not own thesis {}!",
                id
            )));
        }
        let last_version = find_last_version(&state, id).await?;
        match last_version {
            None => {}
            Some(Version { major_num, .. }) if major_num < 1 => {}
            _ => {
                return Err(AppError::Forbidden(format!(
                    "Thesis {} has been public and can only be withdrawn by editors!",
                    id
                )))
            }
        }
    }

    let res = thesis.withdraw_all(state.mongo_db).await?.deleted_count;
    Ok((StatusCode::NO_CONTENT, Json(res)))
}

async fn upload<'a, 'b>(bucket: &'b GridFsBucket, field: Field<'a>) -> Result<ObjectId, AppError> {
    let file_name = field
        .file_name()
        .unwrap_or(
            field
                .name()
                .ok_or(AppError::BadRequest("Empty file name!".to_string()))?,
        )
        .to_string();
    let file_size = if let Some(file_size) = field.size_hint().1 {
        Some(
            u32::try_from(file_size)
                .map_err(|_| AppError::BadRequest(format!("{}-byte is too long!", file_size)))?,
        )
    } else {
        None
    };
    let file_meta = doc! {
        "Content-Type": field.content_type().unwrap_or_default().to_string()
    };

    let source = field
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))
        .into_async_read();
    let res = bucket
        .upload_from_futures_0_3_reader(
            file_name,
            source,
            GridFsUploadOptions::builder()
                .chunk_size_bytes(file_size)
                .metadata(file_meta)
                .build(),
        )
        .await?;
    Ok(res)
}

#[debug_handler]
async fn commit(
    auth_info: AuthInfo,
    State(state): State<AppState>,
    Path(id): Path<ObjectId>,
    mut body: Multipart,
) -> Result<(StatusCode, Json<ObjectId>), AppError> {
    let thesis = find_thesis_by_id(&state, id).await?;
    if !thesis.author_ids.contains(&auth_info.id()?) {
        return Err(AppError::Forbidden(format!(
            "You are not an author of thesis {}!",
            id
        )));
    }

    let bucket = state.mongo_db.gridfs_bucket(None);
    let (file_id, source_id) =
        if let Some(field) = body.next_field().await.map_err(anyhow::Error::from)? {
            (
                upload(&bucket, field).await?,
                if let Some(field) = body.next_field().await.map_err(anyhow::Error::from)? {
                    Some(upload(&bucket, field).await?)
                } else {
                    None
                },
            )
        } else {
            return Err(AppError::BadRequest("No release file!".to_string()));
        };

    let version = Version {
        thesis_id: id,
        uploaded_at: chrono::Utc::now(),
        uploader_id: Some(auth_info.id()?),
        file_id,
        source_id,
        ..Default::default()
    };
    let res = state
        .mongo_db
        .repository::<Version>()
        .insert_one(version, None)
        .await?
        .inserted_id
        .as_object_id()
        .ok_or(anyhow::anyhow!("Cannot get inserted id!"))?;
    Ok((StatusCode::CREATED, Json(res)))
}

pub(super) fn new() -> Router<AppState> {
    Router::new()
        .route("/", routing::post(post).get(gets))
        .route("/:id", routing::get(get).put(put).delete(delete))
        .route("/:id/commit", routing::post(commit))
}
