use super::{ConfigError, ProxyMode, validate_proxy_config};

#[test]
fn accepts_https_origin() {
    let config = validate_proxy_config(
        "https://registry.example.com".into(),
        "127.0.0.1:4873".into(),
        false,
        "enforce".into(),
    )
    .expect("https origin should be accepted");

    assert_eq!(config.proxy_origin, "https://registry.example.com");
    assert_eq!(config.listen_addr, "127.0.0.1:4873");
}

#[test]
fn rejects_http_origin_without_insecure_flag() {
    let result = validate_proxy_config(
        "http://localhost:4873".into(),
        "127.0.0.1:4873".into(),
        false,
        "enforce".into(),
    );

    match result {
        Ok(_) => panic!("http origin should be rejected without insecure flag"),
        Err(ConfigError::ProxyOriginNotHttps { origin }) => {
            assert_eq!(origin, "http://localhost:4873");
        }
        Err(ConfigError::ProxyOriginMissing) => panic!("expected proxy origin scheme error"),
        Err(ConfigError::ProxyModeInvalid { .. }) => panic!("expected proxy origin scheme error"),
    }
}

#[test]
fn accepts_http_origin_with_insecure_flag() {
    let config = validate_proxy_config(
        "http://localhost:4873".into(),
        "127.0.0.1:4873".into(),
        true,
        "enforce".into(),
    )
    .expect("http origin should be accepted with insecure flag");

    assert_eq!(config.proxy_origin, "http://localhost:4873");
}

#[test]
fn rejects_http_origin_when_insecure_override_is_false() {
    let result = validate_proxy_config(
        "http://localhost:4873".into(),
        "127.0.0.1:4873".into(),
        false,
        "enforce".into(),
    );

    assert!(matches!(
        result,
        Err(ConfigError::ProxyOriginNotHttps { .. })
    ));
}

#[test]
fn rejects_empty_origin_as_not_https() {
    let result = validate_proxy_config("".into(), "127.0.0.1:4873".into(), false, "enforce".into());

    match result {
        Ok(_) => panic!("empty origin should be rejected"),
        Err(ConfigError::ProxyOriginNotHttps { origin }) => {
            assert_eq!(origin, "");
        }
        Err(ConfigError::ProxyOriginMissing) => panic!("expected proxy origin scheme error"),
        Err(ConfigError::ProxyModeInvalid { .. }) => panic!("expected proxy origin scheme error"),
    }
}

#[test]
fn rejects_ftp_origin_without_insecure_flag() {
    let result = validate_proxy_config(
        "ftp://example.com".into(),
        "127.0.0.1:4873".into(),
        false,
        "enforce".into(),
    );

    match result {
        Ok(_) => panic!("ftp origin should be rejected without insecure flag"),
        Err(ConfigError::ProxyOriginNotHttps { origin }) => {
            assert_eq!(origin, "ftp://example.com");
        }
        Err(ConfigError::ProxyOriginMissing) => panic!("expected proxy origin scheme error"),
        Err(ConfigError::ProxyModeInvalid { .. }) => panic!("expected proxy origin scheme error"),
    }
}

#[test]
fn preserves_custom_listen_addr() {
    let config = validate_proxy_config(
        "https://registry.example.com".into(),
        "0.0.0.0:8080".into(),
        false,
        "enforce".into(),
    )
    .expect("https origin should be accepted");

    assert_eq!(config.listen_addr, "0.0.0.0:8080");
}

#[test]
fn accepts_enforce_mode() {
    let config = validate_proxy_config(
        "https://registry.example.com".into(),
        "127.0.0.1:4873".into(),
        false,
        "enforce".into(),
    )
    .expect("enforce mode should be accepted");

    assert_eq!(config.mode, ProxyMode::Enforce);
}

#[test]
fn accepts_audit_mode() {
    let config = validate_proxy_config(
        "https://registry.example.com".into(),
        "127.0.0.1:4873".into(),
        false,
        "audit".into(),
    )
    .expect("audit mode should be accepted");

    assert_eq!(config.mode, ProxyMode::Audit);
}

#[test]
fn rejects_invalid_proxy_mode() {
    let result = validate_proxy_config(
        "https://registry.example.com".into(),
        "127.0.0.1:4873".into(),
        false,
        "invalid-mode".into(),
    );

    match result {
        Ok(_) => panic!("invalid mode should be rejected"),
        Err(ConfigError::ProxyModeInvalid { value }) => {
            assert_eq!(value, "invalid-mode");
        }
        Err(other) => panic!("expected ProxyModeInvalid, got {other:?}"),
    }
}
