//! Error implementations

use thiserror::Error;
use you_have_mail_common as yhm;

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
    RequestError { msg: String },
    #[error("Account session expired")]
    LoggedOut,
    #[error("Account backend is not reachable")]
    Offline,
    #[error("{error}")]
    Config { error: ConfigError },
    #[error("Proxy settings invalid or Proxy is unreachable")]
    ProxyError,
    #[error("Unknown: {msg}")]
    Unknown { msg: String },
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Backend '{backend}' for account '{account}' was not found")]
    BackendNotFound { account: String, backend: String },
    #[error(
    "An error occurred while deserializing auth info for '{backend}' with account '{account}': {error}"
    )]
    BackendConfig {
        account: String,
        backend: String,
        error: String,
    },
    #[error("An encryption/decryption occurred: {msg}")]
    Crypto { msg: String },
    #[error("A JSON serialization/deserialization error occurred: {msg}")]
    JSON { msg: String },
    #[error("RPC failed: {msg}")]
    RPCFailed { msg: String },
}

impl From<yhm::backend::BackendError> for ServiceError {
    fn from(value: yhm::backend::BackendError) -> Self {
        use yhm::backend::BackendError;
        match value {
            BackendError::LoggedOut => ServiceError::LoggedOut,
            BackendError::Offline => ServiceError::Offline,
            BackendError::Request(e) => ServiceError::RequestError { msg: e.to_string() },
            BackendError::Unknown(e) => ServiceError::Unknown { msg: e.to_string() },
        }
    }
}

impl From<yhm::AccountError> for ServiceError {
    fn from(value: yhm::AccountError) -> Self {
        match value {
            yhm::AccountError::InvalidState => ServiceError::InvalidAccountState,
            yhm::AccountError::Backend(e) => e.into(),
            yhm::AccountError::Proxy => ServiceError::ProxyError,
        }
    }
}

impl From<yhm::ObserverError> for ServiceError {
    fn from(value: yhm::ObserverError) -> Self {
        match value {
            yhm::ObserverError::AccountError(e) => e.into(),
            yhm::ObserverError::Unknown(e) => ServiceError::Unknown { msg: e.to_string() },
            yhm::ObserverError::AccountAlreadyActive(a) => ServiceError::AccountAlreadyActive {
                email: a.email().to_string(),
            },
            yhm::ObserverError::NoSuchAccount(e) => ServiceError::AccountNotFound { email: e },
        }
    }
}

impl<T, E: Into<ServiceError>> From<yhm::ObserverRPCError<T, E>> for ServiceError {
    fn from(value: yhm::ObserverRPCError<T, E>) -> Self {
        match value {
            yhm::ObserverRPCError::Error(e) => e.into(),
            yhm::ObserverRPCError::SendFailed(_)
            | yhm::ObserverRPCError::SendFailedUnexpectedType => ServiceError::RPCFailed {
                msg: "Failed to send RPC request".to_string(),
            },
            yhm::ObserverRPCError::NoReply => ServiceError::RPCFailed {
                msg: "Received no reply to request".to_string(),
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

impl<T, E: Into<ConfigError>> From<yhm::ObserverRPCError<T, E>> for ConfigError {
    fn from(value: yhm::ObserverRPCError<T, E>) -> Self {
        match value {
            yhm::ObserverRPCError::Error(e) => e.into(),
            yhm::ObserverRPCError::SendFailed(_)
            | yhm::ObserverRPCError::SendFailedUnexpectedType => ConfigError::RPCFailed {
                msg: "Failed to send RPC request".to_string(),
            },
            yhm::ObserverRPCError::NoReply => ConfigError::RPCFailed {
                msg: "Received no reply to request".to_string(),
            },
        }
    }
}

impl From<yhm::ConfigLoadError> for ConfigError {
    fn from(value: yhm::ConfigLoadError) -> Self {
        match value {
            yhm::ConfigLoadError::BackendNotFound { account, backend } => {
                ConfigError::BackendNotFound { account, backend }
            }
            yhm::ConfigLoadError::BackendConfig {
                account,
                backend,
                error,
            } => ConfigError::BackendConfig {
                account,
                backend,
                error: error.to_string(),
            },
            yhm::ConfigLoadError::JSON(e) => ConfigError::JSON { msg: e.to_string() },
        }
    }
}

impl From<yhm::ConfigGenError> for ConfigError {
    fn from(value: yhm::ConfigGenError) -> Self {
        match value {
            yhm::ConfigGenError::BackendConfig { account, error } => ConfigError::BackendConfig {
                account,
                backend: String::new(),
                error: error.to_string(),
            },
            yhm::ConfigGenError::JSON(e) => ConfigError::JSON { msg: e.to_string() },
        }
    }
}

impl From<ConfigError> for ServiceError {
    fn from(value: ConfigError) -> Self {
        ServiceError::Config { error: value }
    }
}
