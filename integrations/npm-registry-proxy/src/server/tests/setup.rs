use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::rustls::pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer};

use crate::server::{AppState, build_router};
use crate::upstream::UpstreamFetcher;

pub(super) async fn spawn_proxy_server(
    upstream_registry_url: &str,
) -> (String, tokio::task::JoinHandle<()>) {
    let fetcher =
        UpstreamFetcher::new_with_danger_certs_for_testing(upstream_registry_url).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let proxy_origin = format!("http://localhost:{port}");
    let state = AppState::new(fetcher, proxy_origin);

    let server_handle = tokio::spawn(async move {
        axum::serve(listener, build_router(state)).await.unwrap();
    });

    (format!("http://127.0.0.1:{port}"), server_handle)
}

pub(super) async fn spawn_upstream_https_server(
    response_body: &'static [u8],
) -> (String, tokio::task::JoinHandle<String>) {
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
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n",
            response_body.len()
        );
        socket.write_all(response.as_bytes()).await.unwrap();
        socket.write_all(response_body).await.unwrap();

        String::from_utf8(request_bytes).unwrap()
    });

    (upstream_registry_url, server_handle)
}
