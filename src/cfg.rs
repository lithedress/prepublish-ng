use config::{Config, Environment};
use lettre::message::Mailbox;
use serde::{Deserialize, Serialize};

fn default_sql_db_url() -> String {
    "postgresql://postgres@localhost/prepublish".to_string()
}

fn default_mongo_srv_url() -> String {
    "mongodb://localhost:27017".to_string()
}

fn default_mongo_db_nm() -> String {
    "prepublish".to_string()
}

fn default_srv_addr() -> String {
    "127.0.0.1:8000".to_string()
}

fn default_hash_cost() -> u8 {
    4
}

fn default_mail_box() -> Mailbox {
    "<root@localhost>".parse().unwrap()
}

fn default_relay() -> String {
    "localhost".to_string()
}

fn default_smtp_username() -> String {
    "root".to_string()
}

#[derive(Serialize, Deserialize)]
#[derive(Eq, PartialEq)]
#[derive(Clone)]
#[derive(Debug)]
pub struct AppConfig {
    #[serde(default = "default_sql_db_url")]
    pub(crate) sql_db_url: String,
    #[serde(default = "default_mongo_srv_url")]
    pub(crate) mongo_srv_url: String,
    #[serde(default = "default_mongo_db_nm")]
    pub(crate) mongo_db_nm: String,
    #[serde(default = "default_srv_addr")]
    pub(crate) srv_addr: String,
    #[serde(default = "default_hash_cost")]
    pub(crate) hash_cost: u8,
    #[serde(default = "default_mail_box")]
    pub(crate) sender: Mailbox,
    #[serde(default = "default_relay")]
    pub(crate) relay: String,
    #[serde(default = "default_smtp_username")]
    pub(crate) smtp_username: String,
    #[serde(default)]
    pub(crate) smtp_password: String,
}

impl AppConfig {
    pub(crate) fn new() -> Self {
        let config = Config::builder()
            .add_source(Environment::with_prefix("PREPUBLISH"))
            .build()
            .unwrap();
        config.try_deserialize().unwrap()
    }
}
