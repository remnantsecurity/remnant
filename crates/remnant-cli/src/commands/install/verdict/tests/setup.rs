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

pub(super) async fn spawn_tarball_server(response_body: Vec<u8>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("test server should bind");
    let port = listener
        .local_addr()
        .expect("test server should have a local address")
        .port();

    tokio::spawn(async move {
        let (mut socket, _) = listener
            .accept()
            .await
            .expect("test server should accept a connection");
        let mut request_bytes = Vec::new();
        let mut buffer = [0_u8; 1024];

        loop {
            let bytes_read = socket
                .read(&mut buffer)
                .await
                .expect("test server should read the request");
            request_bytes.extend_from_slice(&buffer[..bytes_read]);

            if request_bytes.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }

        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/octet-stream\r\ncontent-length: {}\r\n\r\n",
            response_body.len()
        );
        socket
            .write_all(response.as_bytes())
            .await
            .expect("test server should write the response header");
        socket
            .write_all(&response_body)
            .await
            .expect("test server should write the response body");
    });

    format!("http://127.0.0.1:{port}/package.tgz")
}

pub(super) async fn spawn_https_packument_server(response_body: Vec<u8>) -> String {
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

    tokio::spawn(async move {
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
            request_bytes.extend_from_slice(&buffer[..bytes_read]);

            if request_bytes.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }

        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n",
            response_body.len()
        );
        socket
            .write_all(response.as_bytes())
            .await
            .expect("test server should write the response header");
        socket
            .write_all(&response_body)
            .await
            .expect("test server should write the response body");
    });

    upstream_registry
}

pub(super) fn packument_fallback_fetcher(upstream_registry: &str) -> UpstreamFetcher {
    UpstreamFetcher::with_certificate_verification_disabled(upstream_registry)
        .expect("self-signed upstream registry URL should be accepted")
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

pub(super) fn unbound_local_url() -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("test port should bind");
    let port = listener
        .local_addr()
        .expect("test port should have an address")
        .port();
    drop(listener);
    format!("http://127.0.0.1:{port}/package.tgz")
}
