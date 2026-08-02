use flate2::Compression;
use flate2::write::GzEncoder;
use std::io;
use std::path::PathBuf;
use tar::{Builder, Header};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

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
