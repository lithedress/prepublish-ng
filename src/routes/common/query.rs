use axum::http::{header, HeaderName, HeaderValue};
use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct AppQuery {
    pub(crate) offset: u64,
    pub(crate) limit: i64,
}

impl AppQuery {
    pub(crate) fn pagenate(&self, count: u64) -> [(HeaderName, HeaderValue); 4] {
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static(mime::APPLICATION_JSON.as_ref()),
            ),
            (
                HeaderName::from_static("X-Pagination-Count"),
                HeaderValue::from_str(&count.to_string()).unwrap(),
            ),
            (
                HeaderName::from_static("X-Pagination-Offset"),
                HeaderValue::from_str(&self.offset.to_string()).unwrap(),
            ),
            (
                HeaderName::from_static("X-Pagination-Limit"),
                HeaderValue::from_str(&self.limit.to_string()).unwrap(),
            ),
        ]
    }
}
