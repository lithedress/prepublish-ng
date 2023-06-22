use crate::mongo_entities::thesis::{Review, VersionState};
use crate::routes::common::auth::{AuthInfo, Permission};
use crate::routes::common::err::AppError;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::{debug_handler, routing, Json, Router};
use mongodm::prelude::ObjectId;
use mongodm::{doc, ToRepository};

#[debug_handler]
async fn get(
    auth_info: AuthInfo,
    State(state): State<AppState>,
    Path(id): Path<ObjectId>,
) -> Result<Json<Review>, AppError> {
    let res = state
        .mongo_db
        .repository::<Review>()
        .find_one(
            doc! {
                "_id": id
            },
            None,
        )
        .await?
        .ok_or(AppError::NotFound(format!(
            "Review with id {} does not exist!",
            id
        )))?;
    if !(auth_info.permitted(Permission::Publishing) || res.reviewer_id == Some(auth_info.id)) {
        let version = super::version::find_version_by_id(&state, res.version_id).await?;
        match version.state {
            VersionState::History | VersionState::Passed(true) => {
                if !(version.uploader_id == Some(auth_info.id)
                    || version
                        .review_state
                        .remainder_reviewer_ids
                        .contains(&auth_info.id))
                {
                    let thesis =
                        super::thesis::find_thesis_by_id(&state, version.thesis_id).await?;
                    if !(thesis.owner_id == auth_info.id
                        || thesis.author_ids.contains(&auth_info.id))
                    {
                        return Err(AppError::Forbidden(format!("Review {} is not public!", id)));
                    }
                }
            }
            _ => {}
        }
    }
    Ok(Json(res))
}

pub(super) fn new() -> Router<AppState> {
    Router::new().route("/:id", routing::get(get))
}
