use crate::domain::{SecretString, UserUid};
use parking_lot::RwLock;
use serde::{Deserialize, Deserializer};
use std::sync::Arc;

/// Authentication token for access to protected API endpoints.
#[derive(Clone)]
pub struct AuthToken(pub SecretString);

impl AuthToken {
    pub fn new(token: SecretString) -> Self {
        Self(token)
    }
}

impl<'de> Deserialize<'de> for AuthToken {
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

/// Authentication data for a [`crate::Session`]
#[derive(Clone)]
pub struct Auth {
    /// Id of the session.
    pub uid: UserUid,
    /// Authentication token.
    pub auth_token: AuthToken,
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
