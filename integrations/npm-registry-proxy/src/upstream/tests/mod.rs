use futures_util::{StreamExt, stream};
use reqwest::Url;

use super::*;

#[tokio::test]
async fn rejects_body_before_buffering_more_than_byte_limit() {
    let chunk = bytes::Bytes::from_static(b"abcdef");
    let body_stream = stream::iter(vec![Ok(chunk)]).boxed();

    let error = read_response_body_with_limit(body_stream, 5)
        .await
        .err()
        .unwrap();

    assert_eq!(
        error,
        FetchPackumentError::BodyByteLimitExceeded { limit: 5 }
    );
}

#[tokio::test]
async fn accepts_body_at_exact_byte_limit() {
    let body_stream = stream::iter(vec![
        Ok(bytes::Bytes::from_static(b"abc")),
        Ok(bytes::Bytes::from_static(b"def")),
    ])
    .boxed();

    let bytes = read_response_body_with_limit(body_stream, 6).await.unwrap();

    assert_eq!(bytes, b"abcdef");
}

#[test]
fn rejects_response_body_that_is_not_valid_json() {
    let error = validate_packument_json_object(b"not json").err().unwrap();

    assert_eq!(error, FetchPackumentError::ResponseBodyNotValidJson);
}

#[test]
fn rejects_json_response_body_whose_root_is_not_object() {
    let error = validate_packument_json_object(br#"["not", "object"]"#)
        .err()
        .unwrap();

    assert_eq!(error, FetchPackumentError::ResponseBodyRootIsNotObject);
}

#[test]
fn builds_scoped_package_url_as_single_encoded_path_segment() {
    let upstream_registry = Url::parse("https://registry.npmjs.org").unwrap();

    let url = build_packument_url(&upstream_registry, "@babel/core").unwrap();

    assert_eq!(url.as_str(), "https://registry.npmjs.org/@babel%2Fcore");
}

#[test]
fn preserves_configured_upstream_registry_path_prefix() {
    let upstream_registry = Url::parse("https://registry.example.test/npm/").unwrap();

    let url = build_packument_url(&upstream_registry, "left-pad").unwrap();

    assert_eq!(url.as_str(), "https://registry.example.test/npm/left-pad");
}
