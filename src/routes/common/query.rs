use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct AppQuery {
    pub(crate) offset: u64,
    pub(crate) limit: i64,
}