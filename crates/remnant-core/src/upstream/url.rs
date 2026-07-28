use std::time::Duration;

use reqwest::Url;

use super::FetchPackumentError;

pub(super) const DEFAULT_UPSTREAM_REGISTRY: &str = "https://registry.npmjs.org";
pub(super) const INSTALL_V1_ACCEPT: &str = "application/vnd.npm.install-v1+json";
pub(super) const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
pub(super) const TOTAL_FETCH_TIMEOUT: Duration = Duration::from_secs(30);

pub(super) fn parse_upstream_registry(upstream_registry: &str) -> Result<Url, FetchPackumentError> {
    let upstream_registry = Url::parse(upstream_registry)
        .map_err(|_| FetchPackumentError::InvalidUpstreamRegistry(upstream_registry.to_string()))?;

    if upstream_registry.scheme() != "https" {
        return Err(FetchPackumentError::UpstreamRegistrySchemeNotHttps);
    }

    Ok(upstream_registry)
}

pub(super) fn build_packument_url(
    upstream_registry: &Url,
    package_name: &str,
) -> Result<Url, FetchPackumentError> {
    let mut url = upstream_registry.clone();
    {
        let mut path_segments = url.path_segments_mut().map_err(|_| {
            FetchPackumentError::InvalidUpstreamRegistry(upstream_registry.to_string())
        })?;

        // Callers must pass an already-validated, URL-safe package name (no leading
        // slashes) — this crate does not perform npm package name validation itself;
        // that responsibility stays with each ecosystem-specific consumer (see
        // docs/decisions/0051-proactive-install-inspection-architecture.md). push()
        // encodes only the '/' in scoped names (@scope/name → @scope%2Fname).
        path_segments.pop_if_empty().push(package_name);
    }

    Ok(url)
}
