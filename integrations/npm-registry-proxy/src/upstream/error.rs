use std::fmt;

use reqwest::StatusCode;

#[derive(Debug, PartialEq, Eq)]
pub enum FetchPackumentError {
    InvalidUpstreamRegistry(String),
    ConnectionTimeout,
    TotalFetchTimeout,
    RedirectEncountered(StatusCode),
    BodyByteLimitExceeded { limit: usize },
    NonSuccessStatus(StatusCode),
    // Payload may include the upstream URL; must not be forwarded in client-facing responses.
    UpstreamRequestFailed(String),
    // Payload may include the upstream URL; must not be forwarded in client-facing responses.
    ResponseBodyReadFailed(String),
}

impl fmt::Display for FetchPackumentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FetchPackumentError::InvalidUpstreamRegistry(registry) => {
                write!(formatter, "upstream registry URL is invalid: {registry}")
            }
            FetchPackumentError::ConnectionTimeout => {
                write!(formatter, "timed out connecting to upstream registry")
            }
            FetchPackumentError::TotalFetchTimeout => {
                write!(formatter, "timed out fetching upstream metadata")
            }
            FetchPackumentError::RedirectEncountered(status_code) => {
                write!(
                    formatter,
                    "upstream registry returned redirect status {status_code}"
                )
            }
            FetchPackumentError::BodyByteLimitExceeded { limit } => {
                write!(
                    formatter,
                    "upstream metadata response exceeded {limit} byte limit"
                )
            }
            FetchPackumentError::NonSuccessStatus(status_code) => {
                write!(formatter, "upstream registry returned status {status_code}")
            }
            FetchPackumentError::UpstreamRequestFailed(message) => {
                write!(formatter, "upstream metadata request failed: {message}")
            }
            FetchPackumentError::ResponseBodyReadFailed(message) => {
                write!(
                    formatter,
                    "upstream metadata response read failed: {message}"
                )
            }
        }
    }
}

impl std::error::Error for FetchPackumentError {}
