use std::env;

const DEFAULT_PROXY_LISTEN_ADDR: &str = "127.0.0.1:4873";

pub struct ProxyConfig {
    pub proxy_origin: String,
    pub listen_addr: String,
}

#[derive(Debug)]
pub enum ConfigError {
    ProxyOriginMissing,
    ProxyOriginNotHttps { origin: String },
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::ProxyOriginMissing => write!(
                f,
                "REMNANT_PROXY_ORIGIN is required; \
                 set it to the public base URL of this proxy (e.g. https://registry.example.com)"
            ),
            ConfigError::ProxyOriginNotHttps { origin } => write!(
                f,
                "REMNANT_PROXY_ORIGIN must use https:// scheme, got: {origin}; \
                 set REMNANT_ALLOW_INSECURE_ORIGIN=1 to allow http:// for local development only"
            ),
        }
    }
}

/// Reads proxy startup configuration from environment variables.
/// Call once at startup. Do not call again after the proxy begins serving.
pub fn load_proxy_config() -> Result<ProxyConfig, ConfigError> {
    let origin = env::var("REMNANT_PROXY_ORIGIN").map_err(|_| ConfigError::ProxyOriginMissing)?;
    let listen_addr = env::var("REMNANT_PROXY_LISTEN_ADDR")
        .unwrap_or_else(|_| DEFAULT_PROXY_LISTEN_ADDR.to_string());
    let allow_insecure_origin = env::var("REMNANT_ALLOW_INSECURE_ORIGIN")
        .map(|value| value == "1")
        .unwrap_or(false);

    validate_proxy_config(origin, listen_addr, allow_insecure_origin)
}

/// Pure validation helper - testable without touching env vars.
pub(crate) fn validate_proxy_config(
    origin: String,
    listen_addr: String,
    allow_insecure_origin: bool,
) -> Result<ProxyConfig, ConfigError> {
    if !origin.starts_with("https://") && !allow_insecure_origin {
        return Err(ConfigError::ProxyOriginNotHttps { origin });
    }

    Ok(ProxyConfig {
        proxy_origin: origin,
        listen_addr,
    })
}

#[cfg(test)]
mod tests;
