//! Basic HTTP Protocol abstraction for the Proton API.

use anyhow;
use std::fmt::Debug;
use thiserror::Error;

#[cfg(feature = "http-ureq")]
pub mod ureq_client;

#[cfg(feature = "http-reqwest")]
pub mod reqwest_client;

mod client;
mod proxy;
mod request;
mod response;
mod sequence;

pub use client::*;
pub use proxy::*;
pub use request::*;
pub use response::*;
pub use sequence::*;

pub(crate) const DEFAULT_HOST_URL: &str = "https://mail.proton.me/api";
pub(crate) const DEFAULT_APP_VERSION: &str = "proton-api-rs";
#[allow(unused)] // it is used by the http implementations
pub(crate) const X_PM_APP_VERSION_HEADER: &str = "X-Pm-Appversion";
pub(crate) const X_PM_UID_HEADER: &str = "X-Pm-Uid";
pub(crate) const X_PM_HUMAN_VERIFICATION_TOKEN: &str = "X-Pm-Human-Verification-Token";
pub(crate) const X_PM_HUMAN_VERIFICATION_TOKEN_TYPE: &str = "X-Pm-Human-Verification-Token-Type";

/// HTTP method.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Method {
    Delete,
    Get,
    Put,
    Post,
    Patch,
}

/// Errors that may occur during an HTTP request, mostly related to network.
#[derive(Debug, Error)]
pub enum Error {
    #[error("API Error: {0}")]
    API(#[from] crate::requests::APIError),
    #[error("A redirect error occurred at '{0}: {1}")]
    Redirect(String, #[source] anyhow::Error),
    #[error("Connection timed out")]
    Timeout(#[source] anyhow::Error),
    #[error("Connection error: {0}")]
    Connection(#[source] anyhow::Error),
    #[error("Request/Response body error: {0}")]
    Request(#[source] anyhow::Error),
    #[error("Encoding/Decoding error: {0}")]
    EncodeOrDecode(#[source] anyhow::Error),
    #[error("Unexpected error occurred: {0}")]
    Other(#[source] anyhow::Error),
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self::EncodeOrDecode(value.into())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
