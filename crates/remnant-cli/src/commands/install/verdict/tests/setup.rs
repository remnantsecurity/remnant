use flate2::Compression;
use flate2::write::GzEncoder;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;

use remnant_core::UpstreamFetcher;
use tar::{Builder, Header};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::rustls::pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer};

pub(super) const TARBALL_CONTENT_TYPE: &str = "application/octet-stream";
const PACKUMENT_CONTENT_TYPE: &str = "application/json";

pub(super) fn build_tarball_with_package_json(package_json: &[u8]) -> Vec<u8> {
    let mut buffer = Vec::new();
    {
        let encoder = GzEncoder::new(&mut buffer, Compression::default());
        let mut builder = Builder::new(encoder);
        let mut header = Header::new_gnu();
        header.set_size(package_json.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder
            .append_data(
                &mut header,
                "package/package.json",
                io::Cursor::new(package_json),
            )
            .expect("test package.json entry should be appended");
        let encoder = builder
            .into_inner()
            .expect("tar builder should finish successfully");
        encoder
            .finish()
            .expect("gzip encoder should finish successfully");
    }
    buffer
}

pub(super) async fn spawn_https_server(response_body: Vec<u8>, content_type: &str) -> String {
    let (listener, tls_acceptor, upstream_registry) = bind_https_listener().await;
    let content_type = content_type.to_owned();

    tokio::spawn(async move {
        serve_https_response(&listener, &tls_acceptor, &response_body, &content_type).await;
    });

    upstream_registry
}

pub(super) async fn spawn_https_server_for_two_responses(
    first_body: Vec<u8>,
    second_body: Vec<u8>,
    content_type: &str,
) -> String {
    let (listener, tls_acceptor, upstream_registry) = bind_https_listener().await;
    let content_type = content_type.to_owned();

    tokio::spawn(async move {
        serve_https_response(&listener, &tls_acceptor, &first_body, &content_type).await;
        serve_https_response(&listener, &tls_acceptor, &second_body, &content_type).await;
    });

    upstream_registry
}

pub(super) async fn spawn_https_packument_and_tarball_server(
    tarball_path: &str,
    integrity: &str,
    tarball_bytes: Vec<u8>,
) -> String {
    let (listener, tls_acceptor, upstream_registry) = bind_https_listener().await;
    let packument_body = format!(
        r#"{{"name":"fixture-package","versions":{{"1.0.0":{{"dist":{{"tarball":"{upstream_registry}{tarball_path}","integrity":"{integrity}"}}}}}}}}"#
    )
    .into_bytes();

    tokio::spawn(async move {
        serve_https_response(
            &listener,
            &tls_acceptor,
            &packument_body,
            PACKUMENT_CONTENT_TYPE,
        )
        .await;
        serve_https_response(
            &listener,
            &tls_acceptor,
            &tarball_bytes,
            TARBALL_CONTENT_TYPE,
        )
        .await;
    });

    upstream_registry
}

pub(super) fn self_signed_fixture_fetcher(upstream_registry: &str) -> UpstreamFetcher {
    UpstreamFetcher::with_certificate_verification_disabled(upstream_registry)
        .expect("self-signed upstream registry URL should be accepted")
}

async fn bind_https_listener() -> (TcpListener, TlsAcceptor, String) {
    let certificate = rcgen::generate_simple_self_signed(vec![String::from("localhost")])
        .expect("self-signed certificate should generate");
    let certificate_der = certificate.cert.der().clone();
    let private_key_der = certificate.key_pair.serialize_der();
    let server_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(
            vec![certificate_der],
            PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(private_key_der)),
        )
        .expect("test TLS server config should build");
    let tls_acceptor = TlsAcceptor::from(Arc::new(server_config));
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("test server should bind");
    let upstream_registry = format!(
        "https://localhost:{}",
        listener
            .local_addr()
            .expect("test server should have a local address")
            .port()
    );

    (listener, tls_acceptor, upstream_registry)
}

async fn serve_https_response(
    listener: &TcpListener,
    tls_acceptor: &TlsAcceptor,
    response_body: &[u8],
    content_type: &str,
) {
    let (mut socket, _) = listener
        .accept()
        .await
        .expect("test server should accept a connection");
    let mut socket = tls_acceptor
        .accept(&mut socket)
        .await
        .expect("test server should complete TLS handshake");
    let mut request_bytes = Vec::new();
    let mut buffer = [0_u8; 1024];

    loop {
        let bytes_read = socket
            .read(&mut buffer)
            .await
            .expect("test server should read the request");
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
    socket
        .write_all(response.as_bytes())
        .await
        .expect("test server should write the response header");
    socket
        .write_all(response_body)
        .await
        .expect("test server should write the response body");
}

pub(super) fn unbound_https_upstream_registry_url() -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("test port should bind");
    let port = listener
        .local_addr()
        .expect("test port should have an address")
        .port();
    drop(listener);
    format!("https://127.0.0.1:{port}")
}

pub(super) fn test_temp_path(name: &str) -> PathBuf {
    let root = std::env::current_dir()
        .expect("test should run from a working directory")
        .join("target/remnant-tests/install-verdict");
    std::fs::create_dir_all(&root).expect("test root should be created");
    root.join(name)
}
