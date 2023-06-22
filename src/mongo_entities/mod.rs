use mongodm::prelude::ObjectId;
use utoipa::openapi::{RefOr, Schema};

pub(crate) mod paper_collection;
pub(crate) mod profile;
pub(crate) mod thesis;

pub(crate) struct ObjectIdDef;

impl<'__s> utoipa::ToSchema<'__s> for ObjectIdDef {
    fn schema() -> (&'__s str, RefOr<Schema>) {
        let pattern = regex::Regex::new(r"^[0-9a-f]{24}$").unwrap();
        let example = ObjectId::new().to_hex();
        assert!(pattern.is_match(&example));
        (
            "ObjectId",
            utoipa::openapi::ObjectBuilder::new()
                .property(
                    "$oid",
                    utoipa::openapi::ObjectBuilder::new()
                        .schema_type(utoipa::openapi::SchemaType::String)
                        .description(Some(
                            "ObjectId values are 12 bytes in length, written as hex strings.",
                        ))
                        .max_length(Some(24))
                        .min_length(Some(24))
                        .pattern(Some(pattern.as_str()))
                        .example(Some(example.into())),
                )
                .required("$oid")
                .into(),
        )
    }
}
