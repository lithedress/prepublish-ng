use std::collections::BTreeSet;

use async_recursion::async_recursion;
use mongodm::bson::to_bson;
use mongodm::prelude::{
    MongoCursor, MongoDatabase, MongoDeleteResult, MongoError, MongoFindOneAndUpdateOptions,
    MongoReturnDocument, ObjectId,
};
use mongodm::{
    doc, field, prelude::Set, CollectionConfig, Index, IndexOption, Indexes, Model, ToRepository,
};
use serde::{Deserialize, Serialize};

#[derive(utoipa::ToSchema)]
#[derive(Serialize, Deserialize)]
pub(crate) struct ThesisId {
    #[serde(default)]
    pub(crate) _id: ObjectId,
    #[serde(default)]
    pub(crate) owner_id: ObjectId,
    #[serde(default)]
    pub(crate) is_passed: bool,
    #[serde(default)]
    pub(crate) created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(utoipa::ToSchema)]
#[derive(Serialize, Deserialize)]
pub(crate) struct Thesis {
    #[serde(flatten)]
    pub(crate) id: ThesisId,
    pub(crate) author_ids: Vec<ObjectId>,
    //pub(crate) magazine_id: ObjectId,
    #[serde(default)]
    pub(crate) doi: Option<String>,
    pub(crate) title: String,
    pub(crate) abstraction: String,
    pub(crate) keywords: Vec<String>,
    pub(crate) languages: BTreeSet<String>,
}

impl CollectionConfig for Thesis {
    fn collection_name() -> &'static str {
        "theses"
    }

    fn indexes() -> Indexes {
        Indexes::new()
            .with(
                Index::new(field!(owner_id in ThesisId))
                    .with_key(field!(created_at in ThesisId))
                    .with_option(IndexOption::Unique),
            )
            .with(Index::new(field!(is_passed in ThesisId)))
            .with(Index::new(field!(author_ids in Thesis)))
            //.with(Index::new(field!(magazine_id in Thesis)))
            .with(Index::new(field!(doi in Thesis)))
            .with(Index::new(field!(title in Thesis)))
            .with(Index::new(field!(keywords in Thesis)))
            .with(Index::new(field!(languages in Thesis)))
    }
}

impl Model for Thesis {
    type CollConf = Self;
}

#[derive(utoipa::ToSchema)]
#[derive(Serialize, Deserialize)]
#[derive(Default)]
pub(crate) enum ReviewPattern {
    Editor(#[serde(default)] ObjectId),
    #[default]
    Reviewer,
}

#[derive(utoipa::ToSchema)]
#[derive(Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub(crate) struct ReviewState {
    pub(crate) remainder_reviewer_ids: BTreeSet<ObjectId>,
    pub(crate) pattern: ReviewPattern,
}

#[derive(utoipa::ToSchema)]
#[derive(Serialize, Deserialize)]
#[derive(Default)]
pub(crate) enum VersionState {
    #[default]
    Uploaded,
    Reviewing,
    Passed(bool),
    History,
}

#[derive(utoipa::ToSchema)]
#[derive(Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub(crate) struct Version {
    pub(crate) _id: ObjectId,
    pub(crate) thesis_id: ObjectId,
    pub(crate) uploaded_at: chrono::DateTime<chrono::Utc>,
    pub(crate) uploader_id: Option<ObjectId>,
    pub(crate) major_num: i32,
    pub(crate) minor_num: i32,
    pub(crate) file_id: ObjectId,
    pub(crate) source_id: Option<ObjectId>,
    pub(crate) state: VersionState,
    pub(crate) review_state: ReviewState,
    pub(crate) downloads: i32,
}

impl CollectionConfig for Version {
    fn collection_name() -> &'static str {
        "versions"
    }

    fn indexes() -> Indexes {
        Indexes::new()
            .with(
                Index::new(field!(uploader_id in Version)).with_key(field!(uploaded_at in Version)),
            )
            .with(
                Index::new(field!(thesis_id in Version))
                    .with_key(field!(major_num in Version))
                    .with_key(field!(minor_num in Version))
                    .with_option(IndexOption::Unique),
            )
            .with(Index::new(field!(file_id in Version)))
    }
}

impl Model for Version {
    type CollConf = Self;
}

#[derive(utoipa::ToSchema)]
#[derive(Serialize, Deserialize)]
pub(crate) struct Review {
    #[serde(default)]
    pub(crate) _id: ObjectId,
    #[serde(default)]
    pub(crate) version_id: ObjectId,
    #[serde(default)]
    pub(crate) reviewed_at: chrono::DateTime<chrono::Utc>,
    #[serde(default)]
    pub(crate) reviewer_id: Option<ObjectId>,
    pub(crate) judgement: bool,
    pub(crate) criticism: String,
}

impl CollectionConfig for Review {
    fn collection_name() -> &'static str {
        "reviews"
    }

    fn indexes() -> Indexes {
        Indexes::new()
            .with(Index::new(field!(version_id in Review)))
            .with(Index::new(field!(reviewer_id in Review)).with_key(field!(reviewed_at in Review)))
    }
}

impl Model for Review {
    type CollConf = Self;
}

#[derive(utoipa::ToSchema)]
#[derive(Serialize, Deserialize)]
#[derive(Default)]
pub(crate) enum CommentTargetType {
    #[default]
    Version,
    Comment,
}

#[derive(utoipa::ToSchema)]
#[derive(Serialize, Deserialize)]
pub(crate) struct Comment {
    #[serde(default)]
    pub(crate) _id: ObjectId,
    #[serde(default)]
    pub(crate) poster_id: Option<ObjectId>,
    #[serde(default)]
    pub(crate) posted_at: chrono::DateTime<chrono::Utc>,
    #[serde(default)]
    pub(crate) target_type: CommentTargetType,
    #[serde(default)]
    pub(crate) target_id: ObjectId,
    pub(crate) content: String,
}

impl CollectionConfig for Comment {
    fn collection_name() -> &'static str {
        "comments"
    }

    fn indexes() -> Indexes {
        Indexes::new()
            .with(Index::new(field!(poster_id in Comment)).with_key(field!(posted_at in Comment)))
            .with(Index::new(field!(target_type in Comment)).with_key(field!(target_id in Comment)))
    }
}

impl Model for Comment {
    type CollConf = Self;
}

impl Comment {
    async fn replies(&self, db: MongoDatabase) -> Result<MongoCursor<Comment>, MongoError> {
        db.repository::<Self>()
            .find(
                doc! {
                    field!(target_type in Comment): to_bson(&CommentTargetType::Comment)?,
                    field!(target_id in Comment): self._id
                },
                None,
            )
            .await
    }

    #[async_recursion]
    pub(crate) async fn withdraw(self, db: MongoDatabase) -> Result<MongoDeleteResult, MongoError> {
        let mut replies = self.replies(db.clone()).await?;
        while replies.advance().await? {
            let reply = replies.deserialize_current()?;
            reply.withdraw(db.clone()).await?;
        }
        db.repository::<Self>()
            .delete_many(
                doc! {
                    "_id": self._id
                },
                None,
            )
            .await
    }
}

impl Version {
    async fn comments(&self, db: MongoDatabase) -> Result<MongoCursor<Comment>, MongoError> {
        db.repository::<Comment>()
            .find(
                doc! {
                    field!(target_type in Comment): to_bson(&CommentTargetType::Version)?,
                    field!(target_id in Comment): self._id
                },
                None,
            )
            .await
    }

    pub(crate) async fn reject(self, db: MongoDatabase) -> Result<Option<Self>, MongoError> {
        db.repository::<Self>()
            .find_one_and_update(
                doc! {
                    "_id": self._id
                },
                doc! {
                    Set: {
                        field!(state in Version): to_bson(&VersionState::Passed(false))?
                    }
                },
                MongoFindOneAndUpdateOptions::builder()
                    .return_document(MongoReturnDocument::After)
                    .build(),
            )
            .await
    }

    pub(crate) async fn pass(self, db: MongoDatabase) -> Result<Option<Self>, MongoError> {
        let res = db
            .repository::<Self>()
            .find_one_and_update(
                doc! {
                    "_id": self._id
                },
                doc! {
                    Set: {
                        field!(state in Version): to_bson(&VersionState::Passed(true))?,
                        field!(major_num in Version): self.major_num + 1,
                        field!(minor_num in Version): 0
                    }
                },
                MongoFindOneAndUpdateOptions::builder()
                    .return_document(MongoReturnDocument::After)
                    .build(),
            )
            .await?;
        db.repository::<Self>()
            .update_many(
                doc! {
                    field!(thesis_id in Version): self.thesis_id,
                    field!(major_num in Version): self.major_num
                },
                doc! {
                    Set: {
                        field!(state in Version): to_bson(&VersionState::History)?
                    }
                },
                None,
            )
            .await?;
        db.repository::<Thesis>()
            .find_one_and_update(
                doc! {
                    "_id": self.thesis_id
                },
                doc! {
                    Set: {
                        field!(is_passed in ThesisId): true
                    }
                },
                None,
            )
            .await?;
        Ok(res)
    }

    pub(crate) async fn reviews(
        &self,
        db: MongoDatabase,
    ) -> Result<MongoCursor<Review>, MongoError> {
        db.repository::<Review>()
            .find(
                doc! {
                    field!(version_id in Review): self._id
                },
                None,
            )
            .await
    }

    pub(crate) async fn withdraw(self, db: MongoDatabase) -> Result<MongoDeleteResult, MongoError> {
        db.repository::<Review>()
            .delete_many(
                doc! {
                    field!(version_id in Review): self._id
                },
                None,
            )
            .await?;
        let mut comments = self.comments(db.clone()).await?;
        while comments.advance().await? {
            let comment = comments.deserialize_current()?;
            comment.withdraw(db.clone()).await?;
        }
        db.repository::<Self>()
            .delete_many(
                doc! {
                    "_id": self._id
                },
                None,
            )
            .await
    }
}

impl Thesis {
    pub(crate) async fn withdraw_all(
        self,
        db: MongoDatabase,
    ) -> Result<MongoDeleteResult, MongoError> {
        let mut versions = self.versions(db.clone()).await?;
        while versions.advance().await? {
            let version = versions.deserialize_current()?;
            version.withdraw(db.clone()).await?;
        }
        db.repository::<Self>()
            .delete_many(
                doc! {
                    "_id": self.id._id
                },
                None,
            )
            .await
    }

    async fn versions(&self, db: MongoDatabase) -> Result<MongoCursor<Version>, MongoError> {
        db.repository::<Version>()
            .find(
                doc! {
                    field!(thesis_id in Version): self.id._id
                },
                None,
            )
            .await
    }
}
