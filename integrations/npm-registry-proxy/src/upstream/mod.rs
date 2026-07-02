mod error;
mod limits;
mod url;

use std::env;

use futures_util::StreamExt;
use reqwest::{Client, StatusCode, Url};

use crate::package_name::ValidatedPackageName;

pub use error::{FetchPackumentError, FetchTarballError};
use limits::{
    MAX_PACKUMENT_BYTES, MAX_TARBALL_BYTES, TARBALL_FETCH_TIMEOUT, read_response_body_with_limit,
};
use url::{
    CONNECT_TIMEOUT, DEFAULT_UPSTREAM_REGISTRY, INSTALL_V1_ACCEPT, TOTAL_FETCH_TIMEOUT,
    build_packument_url, parse_upstream_registry,
};

pub struct UpstreamFetcher {
    upstream_registry: Url,
    client: Client,
}

pub struct PackumentResponse {
    pub status_code: StatusCode,
    pub bytes: Vec<u8>,
}

impl UpstreamFetcher {
    pub fn from_env() -> Result<Self, FetchPackumentError> {
        let upstream_registry = env::var("REMNANT_UPSTREAM_REGISTRY")
            .unwrap_or_else(|_| String::from(DEFAULT_UPSTREAM_REGISTRY));

        Self::new(&upstream_registry)
    }

    pub fn new(upstream_registry: &str) -> Result<Self, FetchPackumentError> {
        let upstream_registry = parse_upstream_registry(upstream_registry)?;
        let client = Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|error| FetchPackumentError::UpstreamRequestFailed(error.to_string()))?;

        Ok(Self {
            upstream_registry,
            client,
        })
    }

    pub fn upstream_host(&self) -> &str {
        self.upstream_registry.host_str().unwrap_or("")
    }

    #[cfg(test)]
    pub(crate) fn new_with_danger_certs_for_testing(
        upstream_registry: &str,
    ) -> Result<Self, FetchPackumentError> {
        let upstream_registry = parse_upstream_registry(upstream_registry)?;
        let client = Client::builder()
            .danger_accept_invalid_certs(true)
            .connect_timeout(CONNECT_TIMEOUT)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|error| FetchPackumentError::UpstreamRequestFailed(error.to_string()))?;

        Ok(Self {
            upstream_registry,
            client,
        })
    }

    pub async fn fetch_abbreviated_packument(
        &self,
        package_name: &ValidatedPackageName,
    ) -> Result<PackumentResponse, FetchPackumentError> {
        let url = build_packument_url(&self.upstream_registry, package_name)?;

        let fetch_result = tokio::time::timeout(
            TOTAL_FETCH_TIMEOUT,
            fetch_abbreviated_packument_from_url(&self.client, url),
        )
        .await;

        match fetch_result {
            Ok(result) => result,
            Err(_) => Err(FetchPackumentError::TotalFetchTimeout),
        }
    }

    pub async fn fetch_tarball_bytes(&self, url: &str) -> Result<Vec<u8>, FetchTarballError> {
        let url = Url::parse(url).map_err(|_| FetchTarballError::TarballUrlInvalid)?;

        let fetch_result = tokio::time::timeout(
            TARBALL_FETCH_TIMEOUT,
            fetch_tarball_bytes_from_url(&self.client, url),
        )
        .await;

        match fetch_result {
            Ok(result) => result,
            Err(_) => Err(FetchTarballError::TotalFetchTimeout),
        }
    }
}

async fn fetch_abbreviated_packument_from_url(
    client: &Client,
    url: Url,
) -> Result<PackumentResponse, FetchPackumentError> {
    let response = client
        .get(url)
        .header(reqwest::header::ACCEPT, INSTALL_V1_ACCEPT)
        .send()
        .await
        .map_err(map_request_error)?;

    let status_code = response.status();

    if status_code.is_redirection() {
        return Err(FetchPackumentError::RedirectEncountered(status_code));
    }

    if status_code != StatusCode::OK {
        return Err(FetchPackumentError::NonSuccessStatus(status_code));
    }

    let bytes =
        read_response_body_with_limit(response.bytes_stream().boxed(), MAX_PACKUMENT_BYTES).await?;

    Ok(PackumentResponse { status_code, bytes })
}

fn map_request_error(error: reqwest::Error) -> FetchPackumentError {
    if error.is_timeout() {
        FetchPackumentError::ConnectionTimeout
    } else {
        FetchPackumentError::UpstreamRequestFailed(error.to_string())
    }
}

async fn fetch_tarball_bytes_from_url(
    client: &Client,
    url: Url,
) -> Result<Vec<u8>, FetchTarballError> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(map_tarball_request_error)?;

    let status_code = response.status();

    if status_code != StatusCode::OK {
        return Err(FetchTarballError::NonSuccessStatus(status_code));
    }

    read_response_body_with_limit(response.bytes_stream().boxed(), MAX_TARBALL_BYTES)
        .await
        .map_err(map_tarball_body_error)
}

fn map_tarball_request_error(error: reqwest::Error) -> FetchTarballError {
    if error.is_timeout() {
        FetchTarballError::ConnectionTimeout
    } else {
        FetchTarballError::RequestFailed(error.to_string())
    }
}

fn map_tarball_body_error(error: FetchPackumentError) -> FetchTarballError {
    match error {
        FetchPackumentError::BodyByteLimitExceeded { limit } => {
            FetchTarballError::BodyByteLimitExceeded { limit }
        }
        FetchPackumentError::ResponseBodyReadFailed(message) => {
            FetchTarballError::ResponseBodyReadFailed(message)
        }
        _ => FetchTarballError::ResponseBodyReadFailed(error.to_string()),
    }
}

#[cfg(test)]
mod tests;
