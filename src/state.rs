use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) sql_db: sea_orm::DatabaseConnection,
    pub(crate) mongo_db: mongodm::prelude::MongoDatabase,
    pub(crate) hash_cost: u8,
    pub(crate) sender: Arc<lettre::message::Mailbox>,
    pub(crate) smtp: lettre::AsyncSmtpTransport<lettre::Tokio1Executor>,
}
