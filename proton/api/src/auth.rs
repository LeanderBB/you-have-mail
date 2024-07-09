use crate::domain::SecretString;
use parking_lot::RwLock;
use serde::{Deserialize, Deserializer};
use std::fmt::{Display, Formatter};
use std::sync::Arc;

/// Authentication token for access to protected API endpoints.
#[derive(Clone)]
pub struct Token(pub SecretString);

impl Token {
    #[must_use]
    pub fn new(token: SecretString) -> Self {
        Self(token)
    }
}

impl<'de> Deserialize<'de> for Token {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Self(SecretString::deserialize(deserializer)?))
    }
}

/// Refresh token used to refresh expired session token.
#[derive(Clone)]
pub struct RefreshToken(pub SecretString);

impl RefreshToken {
    #[must_use]
    pub fn new(token: SecretString) -> Self {
        Self(token)
    }
}

impl<'de> Deserialize<'de> for RefreshToken {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Self(SecretString::deserialize(deserializer)?))
    }
}

/// Represents a session id.
#[derive(Debug, Deserialize, Eq, PartialEq, Hash, Clone)]
pub struct Uid(pub(crate) String);

impl Display for Uid {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl secrecy::Zeroize for Uid {
    fn zeroize(&mut self) {
        self.0.zeroize();
    }
}

impl secrecy::CloneableSecret for Uid {}

impl secrecy::DebugSecret for Uid {}

impl AsRef<str> for Uid {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl<T: Into<String>> From<T> for Uid {
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

/// Authentication data for a [`crate::Session`]
#[derive(Clone)]
pub struct Auth {
    /// Id of the session.
    pub uid: Uid,
    /// Authentication token.
    pub auth_token: Token,
    /// Refresh token.
    pub refresh_token: RefreshToken,
}

/// The operation on the store failed.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// Reading operation failed.
    #[error("Read: {0}")]
    Read(anyhow::Error),
    /// Reading operation failed.
    #[error("Write: {0}")]
    Write(anyhow::Error),
    /// Write operation failed.
    #[error("Other: {0}")]
    Other(anyhow::Error),
}

/// Authentication storage interface
pub trait Store {
    /// Retrieve the authentication data, if any.
    ///
    /// # Errors
    /// Returns error if retrieving the data failed.
    fn get(&self) -> Result<Option<&Auth>, StoreError>;

    /// Create or update the authentication data.
    ///
    /// # Errors
    /// Returns error if writing the data failed.
    fn store(&mut self, auth: Auth) -> Result<(), StoreError>;

    /// Delete the authentication data. Can potentially be triggered by logout.
    ///
    /// # Errors
    /// Returns error if writing the data failed.
    fn delete(&mut self) -> Result<(), StoreError>;
}

/// Provides an in Memory authentication storage.
#[derive(Default)]
pub struct InMemoryStore {
    data: Option<Auth>,
}

impl InMemoryStore {
    /// Create a new instance with existing authentication data.
    #[must_use]
    pub fn with(auth: Auth) -> Self {
        Self { data: Some(auth) }
    }
}

impl Store for InMemoryStore {
    fn get(&self) -> Result<Option<&Auth>, StoreError> {
        Ok(self.data.as_ref())
    }

    fn store(&mut self, auth: Auth) -> Result<(), StoreError> {
        self.data = Some(auth);
        Ok(())
    }

    fn delete(&mut self) -> Result<(), StoreError> {
        self.data = None;
        Ok(())
    }
}

pub type ThreadSafeStore = Arc<RwLock<dyn Store>>;

pub fn new_thread_safe_store<T: Store + 'static>(store: T) -> ThreadSafeStore {
    Arc::new(RwLock::new(store))
}
