use std::time::Duration;

use reqwest::Url;

use super::{FetchPackumentError, FetchTarballError};

pub(super) const DEFAULT_UPSTREAM_REGISTRY: &str = "https://registry.npmjs.org";
pub(super) const INSTALL_V1_ACCEPT: &str = "application/vnd.npm.install-v1+json";
pub(super) const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
pub(super) const TOTAL_FETCH_TIMEOUT: Duration = Duration::from_secs(30);

pub(super) fn parse_upstream_registry(upstream_registry: &str) -> Result<Url, FetchPackumentError> {
    let upstream_registry =
        Url::parse(upstream_registry).map_err(|_| FetchPackumentError::InvalidUpstreamRegistry)?;

    if upstream_registry.scheme() != "https" {
        return Err(FetchPackumentError::UpstreamRegistrySchemeNotHttps);
    }

    if !upstream_registry.username().is_empty() || upstream_registry.password().is_some() {
        return Err(FetchPackumentError::UpstreamRegistryContainsCredentials);
    }

    if upstream_registry.fragment().is_some() {
        return Err(FetchPackumentError::UpstreamRegistryContainsFragment);
    }

    Ok(upstream_registry)
}

pub(super) fn build_packument_url(
    upstream_registry: &Url,
    package_name: &str,
) -> Result<Url, FetchPackumentError> {
    let mut url = upstream_registry.clone();
    {
        let mut path_segments = url
            .path_segments_mut()
            .map_err(|_| FetchPackumentError::InvalidUpstreamRegistry)?;

        // Callers must pass an already-validated, URL-safe package name (no leading
        // slashes) — this crate does not perform npm package name validation itself;
        // that responsibility stays with each ecosystem-specific consumer (see
        // docs/decisions/0051-proactive-install-inspection-architecture.md). push()
        // encodes only the '/' in scoped names (@scope/name → @scope%2Fname).
        path_segments.pop_if_empty().push(package_name);
    }

    Ok(url)
}

/// Admits a tarball URL only when it is an `https` URL without credentials or a
/// fragment whose host and effective port equal the configured registry's.
/// Pure: performs no I/O, so rejection happens before any DNS lookup or request.
pub(super) fn parse_admissible_tarball_url(
    upstream_registry: &Url,
    tarball_url: &str,
) -> Result<Url, FetchTarballError> {
    let tarball_url = Url::parse(tarball_url).map_err(|_| FetchTarballError::TarballUrlInvalid)?;

    if tarball_url.scheme() != "https" {
        return Err(FetchTarballError::TarballUrlSchemeNotHttps);
    }

    if !tarball_url.username().is_empty() || tarball_url.password().is_some() {
        return Err(FetchTarballError::TarballUrlContainsCredentials);
    }

    if tarball_url.fragment().is_some() {
        return Err(FetchTarballError::TarballUrlContainsFragment);
    }

    if tarball_url.host() != upstream_registry.host() {
        return Err(FetchTarballError::TarballUrlHostNotConfiguredRegistry);
    }

    if tarball_url.port_or_known_default() != upstream_registry.port_or_known_default() {
        return Err(FetchTarballError::TarballUrlPortNotConfiguredRegistry);
    }

    Ok(tarball_url)
}
