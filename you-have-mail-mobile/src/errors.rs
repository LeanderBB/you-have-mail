//! Error implementations

use thiserror::Error;
use you_have_mail_common as yhm;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum RequestErrorCategory {
    Timeout,
    Connection,
    Request,
    API,
}

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("RPC failed: {msg}")]
    RPCFailed { msg: String },
    #[error("Account {email} not found")]
    AccountNotFound { email: String },
    #[error("The account {email} is already active")]
    AccountAlreadyActive { email: String },
    #[error("The account can't complete this operation in its current state")]
    InvalidAccountState,
    #[error("Backend Error: {msg}")]
    RequestError {
        category: RequestErrorCategory,
        msg: String,
    },
    #[error("Account session expired")]
    LoggedOut,
    #[error("{error}")]
    Config { error: ConfigError },
    #[error("Proxy settings invalid or Proxy is unreachable")]
    ProxyError,
    #[error("Backend Requested Human Verification with Captcha")]
    HVCaptchaRequest { msg: String },
    #[error("Supplied Human Verification Data is not valid: {msg}")]
    HVDataInvalid { msg: String },
    #[error("Encode or Decode error: {msg}")]
    EncodeOrDecode { msg: String },
    #[error("Unknown: {msg}")]
    Unknown { msg: String },
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("{msg}")]
    Crypto { msg: String },
    #[error("{msg}")]
    JSON { msg: String },
    #[error("{msg}")]
    IO { msg: String },
    #[error("{msg}")]
    Unknown { msg: String },
}

impl From<yhm::backend::BackendError> for ServiceError {
    fn from(value: yhm::backend::BackendError) -> Self {
        use yhm::backend::BackendError;
        match value {
            BackendError::LoggedOut(_) => ServiceError::LoggedOut,
            BackendError::Timeout(e) => ServiceError::RequestError {
                category: RequestErrorCategory::Connection,
                msg: e.to_string(),
            },
            BackendError::Request(e) => ServiceError::RequestError {
                category: RequestErrorCategory::Request,
                msg: e.to_string(),
            },
            BackendError::Unknown(e) => ServiceError::Unknown { msg: e.to_string() },
            BackendError::Connection(e) => ServiceError::RequestError {
                category: RequestErrorCategory::Connection,
                msg: e.to_string(),
            },
            BackendError::API(e) => ServiceError::RequestError {
                category: RequestErrorCategory::API,
                msg: e.to_string(),
            },
            BackendError::HVCaptchaRequest(v) => ServiceError::HVCaptchaRequest { msg: v },
            BackendError::HVDataInvalid(v) => ServiceError::HVDataInvalid { msg: v.to_string() },
            BackendError::EncodeOrDecode(v) => ServiceError::EncodeOrDecode { msg: v.to_string() },
        }
    }
}

impl From<yhm::AccountError> for ServiceError {
    fn from(value: yhm::AccountError) -> Self {
        match value {
            yhm::AccountError::InvalidState => ServiceError::InvalidAccountState,
            yhm::AccountError::Backend(e) => e.into(),
            yhm::AccountError::Proxy => ServiceError::ProxyError,
            yhm::AccountError::Unknown(e) => ServiceError::Unknown { msg: e.to_string() },
        }
    }
}

impl From<yhm::ObserverError> for ServiceError {
    fn from(value: yhm::ObserverError) -> Self {
        match value {
            yhm::ObserverError::AccountError(e) => e.into(),
            yhm::ObserverError::Unknown(e) => ServiceError::Unknown { msg: e.to_string() },
            yhm::ObserverError::AccountNotFound(e) => ServiceError::AccountNotFound { email: e },
            yhm::ObserverError::Config(e) => ServiceError::Config { error: e.into() },
            _ => ServiceError::Unknown {
                msg: "Unknown error".into(),
            },
        }
    }
}

impl From<()> for ServiceError {
    fn from(_: ()) -> Self {
        ServiceError::Unknown {
            msg: "Unknown error occurred".to_string(),
        }
    }
}

impl From<yhm::ConfigError> for ConfigError {
    fn from(value: yhm::ConfigError) -> Self {
        match value {
            yhm::ConfigError::IO(e) => ConfigError::IO { msg: e.to_string() },
            yhm::ConfigError::JSON(e) => ConfigError::JSON { msg: e.to_string() },
            yhm::ConfigError::Crypto(e) => ConfigError::Crypto { msg: e.to_string() },
            yhm::ConfigError::Unknown(e) => ConfigError::Unknown { msg: e.to_string() },
        }
    }
}

impl From<ConfigError> for ServiceError {
    fn from(value: ConfigError) -> Self {
        ServiceError::Config { error: value }
    }
}
