mod profile;

use super::common::{auth::AuthInfoStorage, err::AppError};
use crate::{
    mongo_entities::profile::{Profile, ID},
    sql_entities::{
        account::{self, ActiveModel},
        prelude::Account,
    },
    state::AppState,
};
use axum::{
    debug_handler,
    extract::{Path, State},
    http::StatusCode,
    routing, Json, Router,
};
use mongodm::{bson::to_bson, doc, field, prelude::ObjectId, ToRepository};
use sea_orm::{prelude::Uuid, ActiveModelTrait, ActiveValue, EntityTrait};
use serde::Deserialize;

async fn get_hash(cost: u8, salt: [u8; 16], password: String) -> Result<[u8; 24], AppError> {
    tokio::task::spawn_blocking(move || passwords::hasher::bcrypt(cost, &salt, &password))
        .await?
        .map_err(|e| AppError::AnyHow(anyhow::anyhow!(e)))
}

async fn try_find_account(
    state: &AppState,
    email: &lettre::Address,
) -> Result<Option<account::Model>, AppError> {
    let account = Account::find_by_id(email.to_string())
        .one(&state.sql_db)
        .await?;
    Ok(account)
}

async fn try_find_profile(
    state: &AppState,
    email: &lettre::Address,
) -> Result<Option<Profile>, AppError> {
    let res = state
        .mongo_db
        .repository::<Profile>()
        .find_one(
            doc! {
                field!(email in ID): to_bson(&email)?
            },
            None,
        )
        .await?;
    Ok(res)
}

#[derive(Deserialize)]
struct SignupBody {
    password: String,
    #[serde(flatten)]
    profile: Profile,
}

#[debug_handler]
async fn signup(
    State(state): State<AppState>,
    Json(mut body): Json<SignupBody>,
) -> Result<(StatusCode, Json<ObjectId>), AppError> {
    let account = try_find_account(&state, &body.profile.public_profile.id.email).await?;
    if account.is_some() {
        return Err(AppError::Conflict(format!(
            "Account with {} already exists!",
            body.profile.public_profile.id.email
        )));
    }
    if try_find_profile(&state, &body.profile.public_profile.id.email)
        .await?
        .is_some()
    {
        return Err(anyhow::anyhow!(
            "Profile with {} already exists!",
            body.profile.public_profile.id.email
        )
        .into());
    }
    let salt = passwords::hasher::gen_salt();
    let account = ActiveModel {
        email: ActiveValue::Set(body.profile.public_profile.id.email.clone().to_string()),
        salt: ActiveValue::Set(Uuid::from_bytes(salt)),
        password_hash: ActiveValue::Set(
            get_hash(state.hash_cost, salt, body.password).await?.into(),
        ),
        is_administrator: ActiveValue::Set(Account::find().all(&state.sql_db).await?.is_empty()),
        created_at: ActiveValue::Set(chrono::Utc::now().naive_utc()),
        updated_at: ActiveValue::Set(chrono::Utc::now().naive_utc()),
        is_editor: ActiveValue::Set(false),
    };
    Account::insert(account).exec(&state.sql_db).await?;

    body.profile.public_profile.id._id = ObjectId::new();
    body.profile.public_profile.id.avatar_id = None;
    body.profile.public_profile.id.joining_at = chrono::Utc::now();
    let res = state
        .mongo_db
        .repository::<Profile>()
        .insert_one(body.profile, None)
        .await?
        .inserted_id
        .as_object_id()
        .ok_or(AppError::AnyHow(anyhow::anyhow!("Cannot get inserted id!")))?;
    Ok((StatusCode::CREATED, Json(res)))
}

#[derive(Deserialize)]
struct AppointBody {
    email: lettre::Address,
    auth: AuthBody,
}

#[debug_handler]
async fn appoint(
    State(state): State<AppState>,
    Path(yes): Path<bool>,
    Json(body): Json<AppointBody>,
) -> Result<(), AppError> {
    if !auth(&state, &body.auth).await?.is_administrator {
        return Err(AppError::Forbidden(
            "You are not an administrator!".to_string(),
        ));
    }
    let mut account: ActiveModel = try_find_account(&state, &body.email)
        .await?
        .ok_or(AppError::NotFound(format!(
            "Account with email {} does not exist!",
            body.email
        )))?
        .into();
    account.is_editor = ActiveValue::Set(yes);
    account.update(&state.sql_db).await?;
    Ok(())
}

#[derive(Deserialize)]
struct AuthBody {
    email: lettre::Address,
    password: String,
}

#[debug_handler]
async fn login(
    mut auth_info_storage: AuthInfoStorage,
    State(state): State<AppState>,
    Json(body): Json<AuthBody>,
) -> Result<Json<Profile>, AppError> {
    let account = auth(&state, &body).await?;

    let res = try_find_profile(&state, &body.email)
        .await?
        .ok_or(AppError::AnyHow(anyhow::anyhow!(
            "Your profile has been lost!"
        )))?;
    auth_info_storage.store(
        res.public_profile.id._id,
        account.is_administrator,
        account.is_editor,
    )?;
    Ok(Json(res))
}

async fn auth(state: &AppState, body: &AuthBody) -> Result<account::Model, AppError> {
    let account = try_find_account(state, &body.email)
        .await?
        .ok_or(AppError::NotFound(format!(
            "Account with email {} does not exist!",
            body.email
        )))?;
    let salt = account.salt.into_bytes();
    if get_hash(state.hash_cost, salt, body.password.clone())
        .await?
        .to_vec()
        != account.password_hash
    {
        return Err(AppError::BadRequest("Wrong password!".to_string()));
    }
    Ok(account)
}

#[debug_handler]
async fn logout(mut auth_info_storage: AuthInfoStorage) {
    auth_info_storage.destroy()
}

pub(super) fn new() -> Router<AppState> {
    Router::new()
        .nest("/profile", profile::new())
        .route("/signup", routing::post(signup))
        .route("/appoint/:yes", routing::patch(appoint))
        .route("/login", routing::post(login))
        .route("/logout", routing::delete(logout))
}
