use crate::routes::common::err::AppError;
use async_trait::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum_sessions::{
    async_session::Session,
    extractors::{ReadableSession, WritableSession},
};
use mongodm::prelude::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(Eq, PartialEq)]
#[derive(Copy, Clone)]
#[derive(Debug)]
#[repr(usize)]
pub(crate) enum Permission {
    Managing = 0,
    Publishing = 1,
}

#[derive(Serialize, Deserialize)]
#[derive(Eq, PartialEq)]
#[derive(Clone, Copy)]
#[derive(Debug)]
pub(crate) struct AuthInfo {
    pub(crate) id: ObjectId,
    roles: [bool; 2],
}

impl AuthInfo {
    pub(crate) fn permitted(&self, role: Permission) -> bool {
        self.roles
            .get(role as usize)
            .map(bool::to_owned)
            .unwrap_or_default()
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthInfo
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        Ok(ReadableSession::from_request_parts(parts, state)
            .await?
            .get::<Self>("auth_info")
            .unwrap_or(AuthInfo {
                id: ObjectId::from_bytes([0_u8; 12]),
                roles: [false; 2],
            }))
    }
}

#[derive(Debug)]
pub(crate) struct AuthInfoStorage(WritableSession);

impl AuthInfoStorage {
    pub(crate) fn store(
        &mut self,
        id: ObjectId,
        is_administrator: bool,
        is_editor: bool,
    ) -> Result<(), AppError> {
        self.0.insert(
            "auth_info",
            AuthInfo {
                id,
                roles: [is_administrator, is_editor],
            },
        )?;
        Ok(())
    }
}

impl std::ops::Deref for AuthInfoStorage {
    type Target = tokio::sync::OwnedRwLockWriteGuard<Session>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for AuthInfoStorage {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthInfoStorage
where
    S: Send + Sync,
{
    type Rejection = <WritableSession as FromRequestParts<S>>::Rejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        WritableSession::from_request_parts(parts, state)
            .await
            .map(Self)
    }
}
