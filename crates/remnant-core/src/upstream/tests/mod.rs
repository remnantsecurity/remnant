use futures_util::{StreamExt, stream};
use reqwest::Url;

use super::*;
mod setup;
use setup::spawn_https_server;

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
    let (upstream_registry, server_task) =
        spawn_https_server(response_body, "application/json").await;
    let fetcher =
        UpstreamFetcher::with_certificate_verification_disabled(&upstream_registry).unwrap();

    let response = fetcher
        .fetch_abbreviated_packument("left-pad")
        .await
        .unwrap();
    let request = server_task.await.unwrap();

    assert_eq!(response.status_code, reqwest::StatusCode::OK);
    assert_eq!(response.bytes, response_body);
    assert!(request.starts_with("GET /left-pad HTTP/1.1\r\n"));
    assert!(request.contains("accept: application/vnd.npm.install-v1+json\r\n"));
}

#[test]
fn accepts_https_upstream_registry_url() {
    let upstream_registry = parse_upstream_registry("https://registry.npmjs.org").unwrap();

    assert_eq!(upstream_registry.as_str(), "https://registry.npmjs.org/");
}

#[test]
fn rejects_http_upstream_registry_url() {
    let error = parse_upstream_registry("http://registry.npmjs.org")
        .err()
        .unwrap();

    assert_eq!(error, FetchPackumentError::UpstreamRegistrySchemeNotHttps);
}

#[test]
fn rejects_upstream_registry_url_containing_credentials() {
    let error = parse_upstream_registry("https://user:pass@registry.example.test")
        .err()
        .unwrap();

    assert_eq!(
        error,
        FetchPackumentError::UpstreamRegistryContainsCredentials
    );
}

#[test]
fn rejects_upstream_registry_url_containing_a_fragment() {
    let error = parse_upstream_registry("https://registry.example.test#f")
        .err()
        .unwrap();

    assert_eq!(error, FetchPackumentError::UpstreamRegistryContainsFragment);
}

#[test]
fn reports_upstream_registry_scheme_before_credentials() {
    let error = parse_upstream_registry("http://user:pass@registry.example.test")
        .err()
        .unwrap();

    assert_eq!(error, FetchPackumentError::UpstreamRegistrySchemeNotHttps);
}

#[test]
fn reports_upstream_registry_credentials_before_fragment() {
    let error = parse_upstream_registry("https://user:pass@registry.example.test#f")
        .err()
        .unwrap();

    assert_eq!(
        error,
        FetchPackumentError::UpstreamRegistryContainsCredentials
    );
}

#[test]
fn omits_the_configured_url_from_the_malformed_upstream_registry_error() {
    let error = parse_upstream_registry("https://user:secret@registry.example.test:99999")
        .err()
        .unwrap();

    assert_eq!(error, FetchPackumentError::InvalidUpstreamRegistry);
    assert_eq!(error.to_string(), "upstream registry URL is invalid");
    assert!(!error.to_string().contains("user"));
    assert!(!error.to_string().contains("secret"));
    assert!(!error.to_string().contains("registry.example.test"));
}

#[test]
fn admits_tarball_url_matching_the_configured_registry_authority() {
    let candidate = "https://registry.example.test/left-pad/-/left-pad-1.3.0.tgz";
    let registry = Url::parse("https://registry.example.test").unwrap();

    let url = parse_admissible_tarball_url(&registry, candidate).unwrap();

    assert_eq!(url.as_str(), candidate);
}

#[test]
fn admits_tarball_url_with_explicit_default_https_port() {
    let registry = Url::parse("https://registry.example.test").unwrap();

    parse_admissible_tarball_url(&registry, "https://registry.example.test:443/x").unwrap();
}

#[test]
fn admits_tarball_url_outside_the_configured_registry_path_prefix() {
    let registry = Url::parse("https://registry.example.test/npm/").unwrap();

    parse_admissible_tarball_url(&registry, "https://registry.example.test/other/x.tgz").unwrap();
}

#[test]
fn admits_tarball_url_with_a_query_string() {
    let registry = Url::parse("https://registry.example.test").unwrap();

    parse_admissible_tarball_url(&registry, "https://registry.example.test/x.tgz?cache=1").unwrap();
}

#[test]
fn admits_tarball_url_with_an_uppercase_host() {
    let registry = Url::parse("https://registry.example.test").unwrap();

    let url = parse_admissible_tarball_url(&registry, "HTTPS://REGISTRY.EXAMPLE.TEST/x").unwrap();

    assert_eq!(url.host_str(), Some("registry.example.test"));
}

#[test]
fn admits_tarball_url_with_a_percent_encoded_host() {
    let registry = Url::parse("https://registry.example.test").unwrap();

    let url = parse_admissible_tarball_url(&registry, "https://registry%2Eexample.test/x").unwrap();

    assert_eq!(url.host_str(), Some("registry.example.test"));
}

#[test]
fn admits_loopback_tarball_url_when_the_configured_registry_is_that_loopback_authority() {
    let registry = Url::parse("https://127.0.0.1:8443").unwrap();

    parse_admissible_tarball_url(&registry, "https://127.0.0.1:8443/x").unwrap();
}

#[test]
fn admits_ipv4_shorthand_tarball_url_for_the_equivalent_configured_literal() {
    let registry = Url::parse("https://127.0.0.1:8443").unwrap();

    let url = parse_admissible_tarball_url(&registry, "https://127.1:8443/x").unwrap();

    assert_eq!(url.host_str(), Some("127.0.0.1"));
}

#[test]
fn admits_expanded_ipv6_tarball_url_for_the_equivalent_configured_literal() {
    let registry = Url::parse("https://[::1]:8443").unwrap();

    let url = parse_admissible_tarball_url(&registry, "https://[0:0:0:0:0:0:0:1]:8443/x").unwrap();

    assert_eq!(url.host_str(), Some("[::1]"));
}

#[test]
fn admits_unicode_tarball_host_for_the_equivalent_configured_punycode_host() {
    let registry = Url::parse("https://xn--bcher-kva.example").unwrap();

    let url = parse_admissible_tarball_url(&registry, "https://bücher.example/x").unwrap();

    assert_eq!(url.host_str(), Some("xn--bcher-kva.example"));
}

#[test]
fn admits_tarball_url_on_the_configured_non_default_port() {
    let registry = Url::parse("https://registry.example.test:8443").unwrap();

    parse_admissible_tarball_url(&registry, "https://registry.example.test:8443/x").unwrap();
}

#[test]
fn admits_private_registry_tarball_url_on_the_configured_authority() {
    let registry =
        Url::parse("https://npm.internal.example:8443/artifactory/api/npm/npm-remote/").unwrap();

    parse_admissible_tarball_url(
        &registry,
        "https://npm.internal.example:8443/artifactory/api/npm/npm-remote/left-pad/-/left-pad-1.3.0.tgz",
    )
    .unwrap();
}

#[test]
fn rejects_malformed_tarball_url() {
    let registry = Url::parse("https://registry.example.test").unwrap();

    let error = parse_admissible_tarball_url(&registry, "not a url")
        .err()
        .unwrap();

    assert_eq!(error, FetchTarballError::TarballUrlInvalid);
}

#[test]
fn rejects_non_https_tarball_url() {
    let registry = Url::parse("https://registry.example.test").unwrap();

    let error = parse_admissible_tarball_url(&registry, "http://registry.example.test/x")
        .err()
        .unwrap();

    assert_eq!(error, FetchTarballError::TarballUrlSchemeNotHttps);
}

#[test]
fn rejects_tarball_url_containing_credentials() {
    let registry = Url::parse("https://registry.example.test").unwrap();

    let error =
        parse_admissible_tarball_url(&registry, "https://user:pass@registry.example.test/x")
            .err()
            .unwrap();

    assert_eq!(error, FetchTarballError::TarballUrlContainsCredentials);
}

#[test]
fn rejects_tarball_url_containing_only_a_password() {
    let registry = Url::parse("https://registry.example.test").unwrap();

    let error = parse_admissible_tarball_url(&registry, "https://:pass@registry.example.test/x")
        .err()
        .unwrap();

    assert_eq!(error, FetchTarballError::TarballUrlContainsCredentials);
}

#[test]
fn rejects_tarball_url_containing_a_fragment() {
    let registry = Url::parse("https://registry.example.test").unwrap();

    let error = parse_admissible_tarball_url(&registry, "https://registry.example.test/x#f")
        .err()
        .unwrap();

    assert_eq!(error, FetchTarballError::TarballUrlContainsFragment);
}

#[test]
fn rejects_tarball_url_with_a_different_host() {
    let registry = Url::parse("https://registry.example.test").unwrap();

    let error = parse_admissible_tarball_url(&registry, "https://other.example.test/x")
        .err()
        .unwrap();

    assert_eq!(
        error,
        FetchTarballError::TarballUrlHostNotConfiguredRegistry
    );
}

#[test]
fn rejects_tarball_url_with_a_trailing_dot_host() {
    let registry = Url::parse("https://registry.example.test").unwrap();

    let error = parse_admissible_tarball_url(&registry, "https://registry.example.test./x")
        .err()
        .unwrap();

    assert_eq!(
        error,
        FetchTarballError::TarballUrlHostNotConfiguredRegistry
    );
}

#[test]
fn rejects_tarball_url_with_an_ip_literal_host_for_a_domain_registry() {
    let registry = Url::parse("https://registry.example.test").unwrap();

    let error = parse_admissible_tarball_url(&registry, "https://192.0.2.1/x")
        .err()
        .unwrap();

    assert_eq!(
        error,
        FetchTarballError::TarballUrlHostNotConfiguredRegistry
    );
}

#[test]
fn rejects_loopback_tarball_url_for_a_public_registry() {
    let registry = Url::parse("https://registry.npmjs.org").unwrap();

    let error = parse_admissible_tarball_url(&registry, "https://127.0.0.1/x")
        .err()
        .unwrap();

    assert_eq!(
        error,
        FetchTarballError::TarballUrlHostNotConfiguredRegistry
    );
}

#[test]
fn rejects_public_registry_tarball_url_when_a_private_registry_is_configured() {
    let registry =
        Url::parse("https://npm.internal.example:8443/artifactory/api/npm/npm-remote/").unwrap();

    let error = parse_admissible_tarball_url(
        &registry,
        "https://registry.npmjs.org/left-pad/-/left-pad-1.3.0.tgz",
    )
    .err()
    .unwrap();

    assert_eq!(
        error,
        FetchTarballError::TarballUrlHostNotConfiguredRegistry
    );
}

#[test]
fn rejects_tarball_url_with_a_different_port() {
    let registry = Url::parse("https://registry.example.test").unwrap();

    let error = parse_admissible_tarball_url(&registry, "https://registry.example.test:8443/x")
        .err()
        .unwrap();

    assert_eq!(
        error,
        FetchTarballError::TarballUrlPortNotConfiguredRegistry
    );
}

#[test]
fn rejects_tarball_url_on_the_default_port_when_a_non_default_port_is_configured() {
    let registry = Url::parse("https://registry.example.test:8443").unwrap();

    let error = parse_admissible_tarball_url(&registry, "https://registry.example.test/x")
        .err()
        .unwrap();

    assert_eq!(
        error,
        FetchTarballError::TarballUrlPortNotConfiguredRegistry
    );
}

#[test]
fn reports_scheme_before_credentials() {
    let registry = Url::parse("https://registry.example.test").unwrap();

    let error = parse_admissible_tarball_url(&registry, "http://user:pass@registry.example.test/x")
        .err()
        .unwrap();

    assert_eq!(error, FetchTarballError::TarballUrlSchemeNotHttps);
}

#[test]
fn reports_credentials_before_fragment() {
    let registry = Url::parse("https://registry.example.test").unwrap();

    let error =
        parse_admissible_tarball_url(&registry, "https://user:pass@registry.example.test/x#f")
            .err()
            .unwrap();

    assert_eq!(error, FetchTarballError::TarballUrlContainsCredentials);
}

#[test]
fn reports_credentials_before_host() {
    let registry = Url::parse("https://registry.example.test").unwrap();

    let error = parse_admissible_tarball_url(&registry, "https://user:pass@other.example.test/x")
        .err()
        .unwrap();

    assert_eq!(error, FetchTarballError::TarballUrlContainsCredentials);
}

#[test]
fn reports_fragment_before_host() {
    let registry = Url::parse("https://registry.example.test").unwrap();

    let error = parse_admissible_tarball_url(&registry, "https://other.example.test/x#f")
        .err()
        .unwrap();

    assert_eq!(error, FetchTarballError::TarballUrlContainsFragment);
}

#[test]
fn reports_host_before_port() {
    let registry = Url::parse("https://registry.example.test").unwrap();

    let error = parse_admissible_tarball_url(&registry, "https://other.example.test:8443/x")
        .err()
        .unwrap();

    assert_eq!(
        error,
        FetchTarballError::TarballUrlHostNotConfiguredRegistry
    );
}

#[tokio::test]
async fn rejects_tarball_url_before_attempting_a_request() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let port = listener.local_addr().unwrap().port();
    let cases = [
        (
            format!("https://127.0.0.1:{port}"),
            format!("http://127.0.0.1:{port}/x"),
            FetchTarballError::TarballUrlSchemeNotHttps,
        ),
        (
            format!("https://127.0.0.1:{port}"),
            format!("https://user:pass@127.0.0.1:{port}/x"),
            FetchTarballError::TarballUrlContainsCredentials,
        ),
        (
            format!("https://127.0.0.1:{port}"),
            format!("https://127.0.0.1:{port}/x#f"),
            FetchTarballError::TarballUrlContainsFragment,
        ),
        (
            format!("https://localhost:{port}"),
            format!("https://127.0.0.1:{port}/x"),
            FetchTarballError::TarballUrlHostNotConfiguredRegistry,
        ),
        (
            String::from("https://127.0.0.1"),
            format!("https://127.0.0.1:{port}/x"),
            FetchTarballError::TarballUrlPortNotConfiguredRegistry,
        ),
    ];

    // Every candidate names the bound socket, so a regression that issued the
    // request would complete a TCP handshake into the backlog and make accept()
    // succeed. The ephemeral port is never 443. The structural guarantee is
    // stronger than this test because the helper is a pure function in a module
    // with no Client.
    for (configured, candidate, expected) in cases {
        let fetcher = UpstreamFetcher::new(&configured).unwrap();

        let error = fetcher.fetch_tarball_bytes(&candidate).await.err().unwrap();

        assert_eq!(error, expected, "candidate {candidate}");
        assert_eq!(
            listener.accept().err().map(|error| error.kind()),
            Some(std::io::ErrorKind::WouldBlock),
            "candidate {candidate} reached the listener"
        );
    }
}

#[tokio::test]
async fn preserves_existing_tarball_fetch_behavior_for_an_admitted_url() {
    let (upstream_registry, server_task) =
        spawn_https_server(b"fixture tarball bytes", "application/octet-stream").await;
    let fetcher =
        UpstreamFetcher::with_certificate_verification_disabled(&upstream_registry).unwrap();

    let bytes = fetcher
        .fetch_tarball_bytes(&format!("{upstream_registry}/fixture/-/fixture-1.0.0.tgz"))
        .await
        .unwrap();
    let request = server_task.await.unwrap();

    assert_eq!(bytes, b"fixture tarball bytes");
    assert!(request.starts_with("GET /fixture/-/fixture-1.0.0.tgz HTTP/1.1\r\n"));
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
