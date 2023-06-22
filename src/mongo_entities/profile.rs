use mongodm::prelude::ObjectId;
use mongodm::{field, CollectionConfig, Index, IndexOption, Indexes, Model};
use serde::{Deserialize, Serialize};

#[derive(utoipa::ToSchema)]
#[derive(Serialize, Deserialize)]
pub(crate) struct ID {
    #[serde(default)]
    pub(crate) _id: ObjectId,
    pub(crate) email: lettre::Address,
    #[serde(default)]
    pub(crate) avatar_id: Option<ObjectId>,
    #[serde(default)]
    pub(crate) joining_at: chrono::DateTime<chrono::Utc>,
}

#[derive(utoipa::ToSchema)]
#[derive(Serialize, Deserialize)]
pub(crate) struct PublicProfile {
    #[serde(flatten)]
    pub(crate) id: ID,
    pub(crate) name: String,
}

#[derive(utoipa::ToSchema)]
#[derive(Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Setting {
    pub(crate) email_notice: bool,
    pub(crate) push: bool,
}

impl Default for Setting {
    fn default() -> Self {
        Self {
            email_notice: true,
            push: true,
        }
    }
}

#[derive(utoipa::ToSchema)]
#[derive(Serialize, Deserialize)]
pub(crate) struct Profile {
    #[serde(flatten)]
    pub(crate) public_profile: PublicProfile,
    #[serde(default)]
    pub(crate) setting: Setting,
}

impl CollectionConfig for Profile {
    fn collection_name() -> &'static str {
        "profiles"
    }
    fn indexes() -> Indexes {
        Indexes::new().with(Index::new(field!(email in ID)).with_option(IndexOption::Unique))
    }
}

impl Model for Profile {
    type CollConf = Self;
}
