use std::time::Duration;

use reqwest::Url;

use crate::package_name::ValidatedPackageName;

use super::FetchPackumentError;

pub(super) const DEFAULT_UPSTREAM_REGISTRY: &str = "https://registry.npmjs.org";
pub(super) const INSTALL_V1_ACCEPT: &str = "application/vnd.npm.install-v1+json";
pub(super) const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
pub(super) const TOTAL_FETCH_TIMEOUT: Duration = Duration::from_secs(30);

pub(super) fn parse_upstream_registry(upstream_registry: &str) -> Result<Url, FetchPackumentError> {
    Url::parse(upstream_registry)
        .map_err(|_| FetchPackumentError::InvalidUpstreamRegistry(upstream_registry.to_string()))
}

pub(super) fn build_packument_url(
    upstream_registry: &Url,
    package_name: &ValidatedPackageName,
) -> Result<Url, FetchPackumentError> {
    let mut url = upstream_registry.clone();
    {
        let mut path_segments = url.path_segments_mut().map_err(|_| {
            FetchPackumentError::InvalidUpstreamRegistry(upstream_registry.to_string())
        })?;

        // ValidatedPackageName guarantees URL-safe characters with no leading slashes,
        // so push() encodes only the '/' in scoped names (@scope/name → @scope%2Fname).
        // No defensive trimming or normalization is needed here.
        path_segments.pop_if_empty().push(package_name.as_str());
    }

    Ok(url)
}
