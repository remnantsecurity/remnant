use futures_util::{StreamExt, stream::BoxStream};

use super::FetchPackumentError;

pub(super) const MAX_PACKUMENT_BYTES: usize = 32 * 1024 * 1024;

pub(super) async fn read_response_body_with_limit(
    mut body_stream: BoxStream<'static, Result<bytes::Bytes, reqwest::Error>>,
    limit: usize,
) -> Result<Vec<u8>, FetchPackumentError> {
    let mut bytes = Vec::new();

    while let Some(next_chunk) = body_stream.next().await {
        let chunk = next_chunk
            .map_err(|error| FetchPackumentError::ResponseBodyReadFailed(error.to_string()))?;

        if bytes.len().saturating_add(chunk.len()) > limit {
            return Err(FetchPackumentError::BodyByteLimitExceeded { limit });
        }

        bytes.extend_from_slice(&chunk);
    }

    Ok(bytes)
}
