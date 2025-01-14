use crate::account::{Account, AccountWatcher, FFIAccountTableObserver};
use crate::android::{AccountNotificationIds, StateExtension};
use crate::backend::Backend;
use crate::events::{Action, Event};
use crate::proxy::Proxy;
use crate::watcher::WatchHandle;
use sqlite_watcher::watcher::Watcher;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tracing::error;
use you_have_mail_common as yhm;
use you_have_mail_common::secrecy::ExposeSecret;

static WATCHER: OnceLock<Arc<Watcher>> = OnceLock::new();

pub(crate) fn watcher() -> &'static Arc<Watcher> {
    WATCHER.get_or_init(|| Watcher::new().unwrap())
}

#[derive(Debug, uniffi::Error, thiserror::Error)]
pub enum YhmError {
    #[error("Account '{0}' already exists")]
    AccountAlreadyExist(String),
    #[error("Account '{0}' does not exist")]
    AccountNotFound(String),
    #[error("Backend: {0}")]
    Backend(String),
    #[error("State: {0}")]
    State(String),
    #[error("Backend '{0}' does not exist")]
    BackendNotFound(String),
    #[error("V1 Import: {0}")]
    V1Import(String),
    #[error("Proxy Test: {0}")]
    ProxyTest(String),
}

impl From<yhm::yhm::Error> for YhmError {
    fn from(value: yhm::yhm::Error) -> Self {
        match value {
            yhm::yhm::Error::Backend(e) => Self::Backend(e.to_string()),
            yhm::yhm::Error::State(e) => Self::State(e.to_string()),
            yhm::yhm::Error::AccountNotFound(e) => Self::AccountNotFound(e),
            yhm::yhm::Error::AccountAlreadyExist(e) => Self::AccountAlreadyExist(e),
            yhm::yhm::Error::BackendNotFound(e) => Self::BackendNotFound(e),
        }
    }
}

/// You have mail instance.
#[derive(uniffi::Object)]
pub struct Yhm {
    yhm: yhm::yhm::Yhm,
}

#[uniffi::export]
impl Yhm {
    /// Create a new instance wih the given path and encryption key.
    ///
    /// # Errors
    ///
    /// Returns error if the instance failed to initialize.
    #[uniffi::constructor]
    pub fn new(db_path: String, encryption_key: String) -> Result<Self, YhmError> {
        let key = yhm::encryption::Key::with_base64(encryption_key)
            .map_err(|e| yhm::yhm::Error::from(yhm::state::Error::from(e)))?;
        let state = yhm::state::State::new(PathBuf::from(db_path), key, Arc::clone(watcher()))
            .map_err(yhm::yhm::Error::from)?;

        state.android_init_tables().map_err(|e| {
            error!("Failed to init adroid tables: {e}");
            YhmError::State(e.to_string())
        })?;

        Ok(Self {
            yhm: yhm::yhm::Yhm::new(state),
        })
    }

    /// Creates a new instance without initializing the database.
    ///
    /// # Errors
    ///
    /// Returns error if the instance failed to initialize.
    #[uniffi::constructor]
    pub fn without_db_init(db_path: String, encryption_key: String) -> Result<Self, YhmError> {
        let key = yhm::encryption::Key::with_base64(encryption_key)
            .map_err(|e| yhm::yhm::Error::from(yhm::state::Error::from(e)))?;
        let state =
            yhm::state::State::without_init(PathBuf::from(db_path), key, Arc::clone(watcher()));
        Ok(Self {
            yhm: yhm::yhm::Yhm::new(state),
        })
    }

    /// Get all active backends.
    #[must_use]
    pub fn backends(&self) -> Vec<Arc<Backend>> {
        self.yhm
            .backends()
            .iter()
            .map(|v| Arc::new(Backend(Arc::clone(v))))
            .collect()
    }

    /// Get a backend by `name`.
    #[must_use]
    pub fn backend_with_name(&self, name: &str) -> Option<Arc<Backend>> {
        self.yhm
            .backend_with_name(name)
            .map(|v| Arc::new(Backend(Arc::clone(v))))
    }

    /// Logout account with `email`.
    ///
    /// # Errors
    ///
    /// Returns error if the operation failed.
    pub fn logout(&self, email: &str) -> Result<(), YhmError> {
        Ok(self.yhm.logout(email)?)
    }

    /// Delete account with `email`.
    ///
    /// # Errors
    ///
    /// Returns error if the operation failed.
    pub fn delete(&self, email: &str) -> Result<(), YhmError> {
        Ok(self.yhm.delete(email)?)
    }

    /// Get the current poll interval.
    ///
    /// # Errors
    ///
    /// Returns error if the operation failed.
    pub fn poll_interval(&self) -> Result<u64, YhmError> {
        let interval = self.yhm.poll_interval()?;
        Ok(interval.as_secs())
    }

    /// Set the current poll `interval` in seconds
    ///
    /// # Errors
    ///
    /// Returns error if the operation failed.
    pub fn set_poll_interval(&self, interval: u64) -> Result<(), YhmError> {
        Ok(self.yhm.set_poll_interval(Duration::from_secs(interval))?)
    }

    /// Port configuration from v1.
    ///
    /// # Errors
    ///
    /// Returns error if the operation failed.
    pub fn import_v1(&self, path: String) -> Result<(), YhmError> {
        let path = PathBuf::from(path);
        self.yhm
            .import_v1(&path)
            .map_err(|e| YhmError::V1Import(e.to_string()))
    }

    /// Update `proxy` for account with `email`
    ///
    /// # Errors
    ///
    /// Returns error if the operation failed.
    pub fn update_proxy(&self, email: &str, proxy: Option<Proxy>) -> Result<(), YhmError> {
        Ok(self
            .yhm
            .update_proxy(email, proxy.map(Into::into).as_ref())?)
    }

    /// Poll the accounts and return a list of events.
    ///
    /// # Errors
    ///
    /// Returns error if the operation failed.
    pub fn poll(&self) -> Result<(), YhmError> {
        self.yhm.poll()?;
        Ok(())
    }

    /// Get all accounts.
    ///
    /// # Errors
    ///
    /// Returns error if the operation failed.
    pub fn accounts(&self) -> Result<Vec<Arc<Account>>, YhmError> {
        let accounts = self.yhm.accounts()?;

        Ok(accounts
            .into_iter()
            .map(|v| Arc::new(Account::new(v)))
            .collect())
    }

    /// Get an account with `email`
    ///
    /// # Errors
    ///
    /// Returns error if the operation failed.
    pub fn account(&self, email: &str) -> Result<Option<Arc<Account>>, YhmError> {
        let account = self.yhm.account(email)?;

        Ok(account.map(|v| Arc::new(Account::new(v))))
    }

    /// Get the last events.
    ///
    /// # Errors
    ///
    /// Returns error if the operation failed.
    pub fn last_events(&self) -> Result<Vec<Event>, YhmError> {
        Ok(self
            .yhm
            .last_events()?
            .into_iter()
            .map(Event::from)
            .collect())
    }

    /// Watch available accounts and receive an updated list when any changes
    /// are made.
    ///
    /// # Errors
    ///
    /// Returns error if the registration failed.
    pub fn watch_accounts(
        &self,
        observer: Arc<dyn AccountWatcher>,
    ) -> Result<WatchHandle, YhmError> {
        Ok(self
            .yhm
            .watch_accounts(FFIAccountTableObserver(observer))?
            .into())
    }

    /// Apply the `action` to the account with `email`.
    ///
    /// # Errors
    ///
    /// Returns error if the action failed.
    pub fn apply_action(&self, email: &str, action: Action) -> Result<(), YhmError> {
        let action = action.into();
        Ok(self.yhm.apply_actions(email, [action])?)
    }
}

#[uniffi::export]
impl Yhm {
    /// Get or create the stable notificaiton ids for account with `email`.
    ///
    /// # Errors
    ///
    /// Returns error on failure.
    pub fn android_get_or_create_notification_ids(
        &self,
        email: &str,
    ) -> Result<AccountNotificationIds, YhmError> {
        self.yhm
            .state()
            .android_get_or_create_notification_ids(email)
            .map_err(|e| {
                error!("Failed to create notification ids for {email}: {e}");
                YhmError::State(e.to_string())
            })
    }

    /// Get the next email notification id for account with `email`.
    ///
    /// # Errors
    ///
    /// Returns error on failure.
    pub fn android_next_mail_notification_id(&self, email: &str) -> Result<i32, YhmError> {
        self.yhm
            .state()
            .android_next_mail_notification_id(email)
            .map_err(|e| {
                error!("Failed to get next mail notification id for {email}: {e}");
                YhmError::State(e.to_string())
            })
    }
}

/// Generate a new encryption key.
#[uniffi::export]
#[must_use]
pub fn new_encryption_key() -> String {
    yhm::encryption::Key::new().expose_secret().to_base64()
}

impl Yhm {
    pub(crate) fn instance(&self) -> &yhm::yhm::Yhm {
        &self.yhm
    }
}
