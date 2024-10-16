use crate::domain::SecretString;
use parking_lot::RwLock;
use secrecy::{zeroize, ExposeSecret};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
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
pub struct Uid(pub String);

impl Display for Uid {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl zeroize::Zeroize for Uid {
    fn zeroize(&mut self) {
        self.0.zeroize();
    }
}

impl secrecy::CloneableSecret for Uid {}

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
#[derive(Clone, Deserialize)]
pub struct Auth {
    /// Id of the session.
    pub uid: Uid,
    /// Authentication token.
    pub auth_token: Token,
    /// Refresh token.
    pub refresh_token: RefreshToken,
}

impl Serialize for Auth {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Auth", 3)?;
        state.serialize_field("uid", self.uid.as_ref())?;
        state.serialize_field("auth_token", self.auth_token.0.expose_secret())?;
        state.serialize_field("refresh_token", self.refresh_token.0.expose_secret())?;
        state.end()
    }
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

pub type ThreadSafeStore = Arc<RwLock<dyn Store + Send + Sync>>;

pub fn new_thread_safe_store<T: Store + 'static + Send + Sync>(store: T) -> ThreadSafeStore {
    Arc::new(RwLock::new(store))
}
