use futures_util::{StreamExt, stream};
use reqwest::Url;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use super::*;
use crate::package_name::ValidatedPackageName;

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

#[tokio::test]
async fn fetches_abbreviated_packument_round_trip() {
    let response_body = br#"{"name":"left-pad","versions":{}}"#;
    let (upstream_registry, server_task) = spawn_packument_server(response_body).await;
    let fetcher = UpstreamFetcher::new(&upstream_registry).unwrap();
    let package_name = ValidatedPackageName::parse(String::from("left-pad")).unwrap();

    let response = fetcher
        .fetch_abbreviated_packument(&package_name)
        .await
        .unwrap();
    let request = server_task.await.unwrap();

    assert_eq!(response.status_code, reqwest::StatusCode::OK);
    assert_eq!(response.bytes, response_body);
    assert!(request.starts_with("GET /left-pad HTTP/1.1\r\n"));
    assert!(request.contains("accept: application/vnd.npm.install-v1+json\r\n"));
}

#[test]
fn builds_scoped_package_url_as_single_encoded_path_segment() {
    let upstream_registry = Url::parse("https://registry.npmjs.org").unwrap();
    let package_name = ValidatedPackageName::parse(String::from("@babel/core")).unwrap();

    let url = build_packument_url(&upstream_registry, &package_name).unwrap();

    assert_eq!(url.as_str(), "https://registry.npmjs.org/@babel%2Fcore");
}

#[test]
fn preserves_configured_upstream_registry_path_prefix() {
    let upstream_registry = Url::parse("https://registry.example.test/npm/").unwrap();
    let package_name = ValidatedPackageName::parse(String::from("left-pad")).unwrap();

    let url = build_packument_url(&upstream_registry, &package_name).unwrap();

    assert_eq!(url.as_str(), "https://registry.example.test/npm/left-pad");
}

async fn spawn_packument_server(
    response_body: &'static [u8],
) -> (String, tokio::task::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let upstream_registry = format!("http://{}", listener.local_addr().unwrap());

    let server_task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request_bytes = Vec::new();
        let mut buffer = [0_u8; 1024];

        loop {
            let bytes_read = socket.read(&mut buffer).await.unwrap();
            assert!(
                bytes_read > 0,
                "client closed connection before request completed"
            );
            request_bytes.extend_from_slice(&buffer[..bytes_read]);

            if request_bytes.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }

        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n",
            response_body.len()
        );
        socket.write_all(response.as_bytes()).await.unwrap();
        socket.write_all(response_body).await.unwrap();

        String::from_utf8(request_bytes).unwrap()
    });

    (upstream_registry, server_task)
}
