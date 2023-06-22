use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

pub(crate) enum AppError {
    AnyHow(anyhow::Error),
    BadRequest(String),
    Conflict(String),
    Forbidden(String),
    NotFound(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            Self::AnyHow(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            Self::BadRequest(e) => (StatusCode::BAD_REQUEST, e),
            Self::Conflict(e) => (StatusCode::BAD_REQUEST, e),
            Self::Forbidden(e) => (StatusCode::FORBIDDEN, e),
            Self::NotFound(e) => (StatusCode::NOT_FOUND, e),
        }
        .into_response()
    }
}

impl<T: Into<anyhow::Error>> From<T> for AppError {
    fn from(value: T) -> Self {
        Self::AnyHow(value.into())
    }
}
