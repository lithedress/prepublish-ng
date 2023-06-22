use mongodm::prelude::ObjectId;
use mongodm::{field, CollectionConfig, Index, Indexes, Model};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use chrono::Utc;

#[derive(utoipa::ToSchema)]
#[derive(Serialize, Deserialize)]
pub(crate) struct PaperCollection {
    #[serde(default)]
    pub(crate) _id: ObjectId,
    pub(crate) name: String,
    pub(crate) description: String,
    #[serde(default)]
    pub(crate) others: String,
    #[serde(default)]
    pub(crate) languages: BTreeSet<String>,
}

impl PaperCollection {
    fn indexes() -> Indexes {
        Indexes::new()
            .with(Index::new(field!(name in PaperCollection)))
            .with(Index::new(field!(languages in PaperCollection)))
    }
}

#[derive(utoipa::ToSchema)]
#[derive(Serialize, Deserialize)]
pub(crate) struct Category {
    #[serde(flatten)]
    pub(crate) meta: PaperCollection,
    #[serde(default)]
    pub(crate) owner_id: ObjectId,
    #[serde(default)]
    pub(crate) is_public: bool,
    #[serde(default)]
    pub(crate) sub_category_ids: BTreeSet<ObjectId>,
    #[serde(default)]
    pub(crate) magazine_ids: BTreeSet<ObjectId>,
    #[serde(default)]
    pub(crate) thesis_ids: BTreeSet<ObjectId>,
}

impl CollectionConfig for Category {
    fn collection_name() -> &'static str {
        "categories"
    }

    fn indexes() -> Indexes {
        PaperCollection::indexes().with(Index::new(field!(is_public in Category)))
    }
}

impl Model for Category {
    type CollConf = Self;
}

#[derive(utoipa::ToSchema)]
#[derive(Serialize, Deserialize)]
pub(crate) struct Magazine {
    #[serde(flatten)]
    pub(crate) meta: PaperCollection,
    #[serde(default)]
    pub(crate) abbr: BTreeSet<String>,
    #[serde(default)]
    pub(crate) pages_min: i32,
    #[serde(default)]
    pub(crate) homepage: Option<url::Url>,
    #[serde(default)]
    pub(crate) template_link: Option<url::Url>,
    #[serde(default)]
    pub(crate) community_link: Option<url::Url>,
    #[serde(default)]
    pub(crate) modified_at: chrono::DateTime<Utc>,
}

impl CollectionConfig for Magazine {
    fn collection_name() -> &'static str {
        "magazines"
    }

    fn indexes() -> Indexes {
        PaperCollection::indexes().with(Index::new(field!(abbr in Magazine)))
    }
}

impl Model for Magazine {
    type CollConf = Self;
}
