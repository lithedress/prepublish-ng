use std::str::FromStr;

use axum::body::StreamBody;
use axum::extract::{Path, State};
use axum::headers::{ContentDisposition, ContentLength, ContentType, Header, HeaderValue};
use axum::{routing, Router, TypedHeader};
use futures_codec::{BytesCodec, FramedRead};
use futures_util::{Stream, StreamExt, TryStreamExt};
use mime::Mime;
use mongodm::prelude::ObjectId;
use mongodm::{bson, doc};

use crate::routes::common;
use crate::routes::common::err::AppError;
use crate::state::AppState;

async fn get(
    State(state): State<AppState>,
    Path(id): Path<ObjectId>,
) -> Result<
    (
        TypedHeader<ContentDisposition>,
        TypedHeader<ContentLength>,
        TypedHeader<ContentType>,
        StreamBody<impl Stream<Item = std::io::Result<Vec<u8>>> + Sized>,
    ),
    AppError,
> {
    let db = state.mongo_db.clone();
    let bucket = db.gridfs_bucket(None);

    let doc = bucket
        .find(
            doc! {
                "_id": id
            },
            None,
        )
        .await?
        .next()
        .await
        .ok_or(anyhow::anyhow!("File {} lost!", id))??;
    let content_disposition =
        ContentDisposition::decode(&mut std::iter::once(&HeaderValue::try_from(format!(
            "{}{}{}",
            common::DISPOSITION_PREFIX,
            doc.filename.unwrap_or_default(),
            common::DISPOSITION_SUFFIX
        ))?))?;
    let content_length = ContentLength(doc.chunk_size_bytes.into());
    let content_type = ContentType::from(
        Mime::from_str(
            &doc.metadata
                .and_then(|md| md.get("Content-Type").map(ToString::to_string))
                .unwrap_or_default(),
        )
        .unwrap_or(mime::TEXT_PLAIN),
    );

    let stream = bucket.open_download_stream(bson!(id)).await?;
    let stream = FramedRead::new(stream, BytesCodec).map_ok(|b| b.to_vec());

    Ok((
        TypedHeader(content_disposition),
        TypedHeader(content_length),
        TypedHeader(content_type),
        StreamBody::new(stream),
    ))
}

pub(super) fn new() -> Router<AppState> {
    Router::new().route("/file/:id", routing::get(get))
}
