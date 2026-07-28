use std::sync::Arc;
use std::{fs, io::Cursor, path::PathBuf};

use flate2::Compression;
use flate2::write::GzEncoder;
use remnant_core::UpstreamFetcher;
use tar::{Builder, Header};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::rustls::pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer};

use crate::config::ProxyMode;
use crate::server::{AppState, build_router};

pub(super) async fn spawn_proxy_server(
    upstream_registry_url: &str,
) -> (String, tokio::task::JoinHandle<()>) {
    let fetcher =
        UpstreamFetcher::with_certificate_verification_disabled(upstream_registry_url).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let proxy_origin = format!("http://localhost:{port}");
    let state = AppState::new(
        fetcher,
        proxy_origin,
        String::from("test-version"),
        String::from("test-commit-sha"),
        ProxyMode::Enforce,
    );

    let server_handle = tokio::spawn(async move {
        axum::serve(listener, build_router(state)).await.unwrap();
    });

    (format!("http://127.0.0.1:{port}"), server_handle)
}

pub(super) async fn spawn_proxy_server_with_audit_sink(
    upstream_registry_url: &str,
) -> (
    String,
    tokio::task::JoinHandle<()>,
    tokio::sync::mpsc::UnboundedReceiver<String>,
) {
    let fetcher =
        UpstreamFetcher::with_certificate_verification_disabled(upstream_registry_url).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let proxy_origin = format!("http://localhost:{port}");
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let state = AppState::new(
        fetcher,
        proxy_origin,
        String::from("test-version"),
        String::from("test-commit-sha"),
        ProxyMode::Enforce,
    )
    .with_audit_sink(tx);

    let server_handle = tokio::spawn(async move {
        axum::serve(listener, build_router(state)).await.unwrap();
    });

    (format!("http://127.0.0.1:{port}"), server_handle, rx)
}

pub(super) async fn spawn_proxy_server_with_mode(
    upstream_registry_url: &str,
    mode: ProxyMode,
) -> (String, tokio::task::JoinHandle<()>) {
    let fetcher =
        UpstreamFetcher::with_certificate_verification_disabled(upstream_registry_url).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let proxy_origin = format!("http://localhost:{port}");
    let state = AppState::new(
        fetcher,
        proxy_origin,
        String::from("test-version"),
        String::from("test-commit-sha"),
        mode,
    );

    let server_handle = tokio::spawn(async move {
        axum::serve(listener, build_router(state)).await.unwrap();
    });

    (format!("http://127.0.0.1:{port}"), server_handle)
}

pub(super) async fn spawn_proxy_server_with_mode_and_audit_sink(
    upstream_registry_url: &str,
    mode: ProxyMode,
) -> (
    String,
    tokio::task::JoinHandle<()>,
    tokio::sync::mpsc::UnboundedReceiver<String>,
) {
    let fetcher =
        UpstreamFetcher::with_certificate_verification_disabled(upstream_registry_url).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let proxy_origin = format!("http://localhost:{port}");
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let state = AppState::new(
        fetcher,
        proxy_origin,
        String::from("test-version"),
        String::from("test-commit-sha"),
        mode,
    )
    .with_audit_sink(tx);

    let server_handle = tokio::spawn(async move {
        axum::serve(listener, build_router(state)).await.unwrap();
    });

    (format!("http://127.0.0.1:{port}"), server_handle, rx)
}

pub(super) async fn spawn_upstream_https_server(
    response_body: Vec<u8>,
) -> (String, tokio::task::JoinHandle<String>) {
    let (tls_acceptor, listener, upstream_registry_url) = create_test_tls_listener().await;

    let server_handle = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut socket = tls_acceptor.accept(&mut socket).await.unwrap();
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
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
            response_body.len()
        );
        socket.write_all(response.as_bytes()).await.unwrap();
        socket.write_all(&response_body).await.unwrap();

        String::from_utf8(request_bytes).unwrap()
    });

    (upstream_registry_url, server_handle)
}

pub(super) async fn spawn_upstream_https_server_for_two_requests(
    first_response_body: Vec<u8>,
    first_content_type: &'static str,
    second_response_body: Vec<u8>,
    second_content_type: &'static str,
) -> (String, tokio::task::JoinHandle<()>) {
    let (tls_acceptor, listener, upstream_registry_url) = create_test_tls_listener().await;

    let server_handle = tokio::spawn(async move {
        serve_https_response(
            &listener,
            &tls_acceptor,
            &first_response_body,
            first_content_type,
        )
        .await;
        serve_https_response(
            &listener,
            &tls_acceptor,
            &second_response_body,
            second_content_type,
        )
        .await;
    });

    (upstream_registry_url, server_handle)
}

pub(super) async fn spawn_upstream_https_server_for_packument_and_tarball(
    package_name: &str,
    version: &str,
    tarball_path: &str,
    integrity: Option<&str>,
    tarball_bytes: Vec<u8>,
) -> (String, tokio::task::JoinHandle<()>) {
    let (tls_acceptor, listener, upstream_registry_url) = create_test_tls_listener().await;
    let packument = packument_bytes(
        package_name,
        version,
        &format!("{upstream_registry_url}{tarball_path}"),
        integrity,
    );

    let server_handle = tokio::spawn(async move {
        serve_https_response(&listener, &tls_acceptor, &packument, "application/json").await;
        serve_https_response(
            &listener,
            &tls_acceptor,
            &tarball_bytes,
            "application/octet-stream",
        )
        .await;
    });

    (upstream_registry_url, server_handle)
}

pub(super) fn build_tgz_from_package_json(package_json: &[u8]) -> Vec<u8> {
    let encoder = GzEncoder::new(Vec::new(), Compression::default());
    let mut archive = Builder::new(encoder);
    let mut header = Header::new_gnu();

    header.set_size(package_json.len() as u64);
    header.set_mode(0o644);
    header.set_uid(0);
    header.set_gid(0);
    header.set_mtime(0);
    header.set_cksum();

    archive
        .append_data(
            &mut header,
            "package/package.json",
            Cursor::new(package_json),
        )
        .unwrap();
    let encoder = archive.into_inner().unwrap();
    encoder.finish().unwrap()
}

pub(super) fn read_fixture_package_json(category: &str, name: &str) -> Vec<u8> {
    fs::read(
        fixture_root()
            .join(category)
            .join(name)
            .join("package")
            .join("package.json"),
    )
    .unwrap()
}

async fn create_test_tls_listener() -> (TlsAcceptor, TcpListener, String) {
    let certificate = rcgen::generate_simple_self_signed(vec![String::from("localhost")]).unwrap();
    let certificate_der = certificate.cert.der().clone();
    let private_key_der = certificate.key_pair.serialize_der();
    let server_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(
            vec![certificate_der],
            PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(private_key_der)),
        )
        .unwrap();
    let tls_acceptor = TlsAcceptor::from(Arc::new(server_config));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let upstream_registry_url = format!(
        "https://localhost:{}",
        listener.local_addr().unwrap().port()
    );

    (tls_acceptor, listener, upstream_registry_url)
}

async fn serve_https_response(
    listener: &TcpListener,
    tls_acceptor: &TlsAcceptor,
    response_body: &[u8],
    content_type: &str,
) {
    let (mut socket, _) = listener.accept().await.unwrap();
    let mut socket = tls_acceptor.accept(&mut socket).await.unwrap();
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
        "HTTP/1.1 200 OK\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
        response_body.len()
    );
    socket.write_all(response.as_bytes()).await.unwrap();
    socket.write_all(response_body).await.unwrap();
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("crates")
        .join("remnant-cli")
        .join("fixtures")
}

pub(super) fn packument_bytes(
    package_name: &str,
    version: &str,
    upstream_tarball_url: &str,
    integrity: Option<&str>,
) -> Vec<u8> {
    let mut dist = serde_json::json!({
        "tarball": upstream_tarball_url,
    });

    if let Some(integrity) = integrity {
        dist["integrity"] = serde_json::json!(integrity);
    }

    serde_json::to_vec(&serde_json::json!({
        "name": package_name,
        "versions": {
            version: {
                "dist": dist,
            },
        },
    }))
    .unwrap()
}
