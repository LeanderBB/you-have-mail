//! State management of accounts in the database.

use crate::encryption::Key;
use crate::events::Event;
use chrono::{DateTime, Utc};
use rusqlite::{OptionalExtension, Row};
use secrecy::{ExposeSecret, SecretBox, SecretSlice};
use serde::Serialize;
use serde::de::DeserializeOwned;
use sqlite_rwc::drivers::rusqlite::{
    RusqliteConnectionPool, RusqlitePooledConnection, RusqliteTransaction,
};
use sqlite_rwc::{ConnectionPoolConfig, ConnectionPoolError};
use sqlite_watcher::watcher::{DropRemoveTableObserverHandle, TableObserver, Watcher};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::{Arc, Weak};
use std::time::Duration;
use tracing::error;
use you_have_mail_http::Proxy;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Serialization: {0}")]
    Serialization(#[from] serde_json::error::Error),
    #[error("Crypto: {0}")]
    Crypto(#[from] crate::encryption::Error),
    #[error("Encryption: {0}")]
    Encryption(anyhow::Error),
    #[error("Db: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("DbPool: {0}")]
    DbPool(#[from] ConnectionPoolError<rusqlite::Error>),
    #[error("Other: {0}")]
    Other(anyhow::Error),
}

/// Represents a stored account.
///
/// To add a new account to the system, you must provide an implementation of [`IntoAccount`] when
/// interacting with [`crate::Yhm`].
///
/// Accounts can have a regular non-encrypted state, which can be set using [`set_state`] and a
/// secret encrypted state with [`set_secret`].
///
/// Since the authentication tokens are stored in the secret state, an account is considered logged
/// in if there is a secret value. If no such value is present, it is treated as logged out.
///
#[derive(Clone)]
pub struct Account {
    email: String,
    backend: String,
    last_poll: Option<DateTime<Utc>>,
    state: Arc<State>,
}

impl Account {
    /// Create a new account with `email` and `backend` name.
    #[must_use]
    fn new(
        email: String,
        backend: String,
        last_poll: Option<DateTime<Utc>>,
        state: Arc<State>,
    ) -> Self {
        Self {
            email,
            backend,
            last_poll,
            state,
        }
    }

    /// Get the account's email.
    #[must_use]
    pub fn email(&self) -> &str {
        &self.email
    }

    /// Get the account's backend name.
    #[must_use]
    pub fn backend(&self) -> &str {
        &self.backend
    }

    /// Get the last time this account was polled.
    ///
    /// Returns `None` if never polled.
    #[must_use]
    pub fn last_poll(&self) -> Option<&DateTime<Utc>> {
        self.last_poll.as_ref()
    }

    /// Get the account state.
    ///
    /// # Errors
    ///
    /// Return error if the state construction failed.
    pub fn state<T: DeserializeOwned>(&self) -> Result<Option<T>, Error> {
        self.state.account_state(&self.email)
    }

    /// Get the secret state.
    ///
    /// # Errors
    ///
    /// Return error if the state construction failed.
    pub fn secret<T: DeserializeOwned>(&self) -> Result<Option<T>, Error> {
        self.state.secret_state(&self.email)
    }

    /// Get the proxy configuration.
    ///
    /// # Errors
    ///
    /// Return error if the state construction failed.
    pub fn proxy(&self) -> Result<Option<Proxy>, Error> {
        self.state.proxy(&self.email)
    }

    /// Update the account with new `state`.
    ///
    /// This state is not encrypted. To store state encrypted see [`set_secret`].
    ///
    /// If `state` is `None`, existing state will be erased.
    ///
    /// # Errors
    ///
    /// Return error if the state construction failed.
    pub fn set_state<T: Serialize>(&self, state: Option<&T>) -> Result<(), Error> {
        if let Some(state) = state {
            self.state.set_account_state(&self.email, state)
        } else {
            self.state.delete_account_state(&self.email)
        }
    }

    /// Update the account with new `secret` state.
    ///
    /// Secret state is always stored encrypted. For non encrypted state see [`set_state`].
    ///
    /// If `secret` is `None`, existing secret will be erased.
    ///
    /// # Errors
    ///
    /// Return error if the state construction failed.
    pub fn set_secret<T: Serialize>(&self, secret: Option<&T>) -> Result<(), Error> {
        if let Some(secret) = secret {
            self.state.set_secret_state(&self.email, secret)
        } else {
            self.state.delete_secret_state(&self.email)
        }
    }

    /// Update the account with new `proxy` config.
    ///
    /// # Errors
    ///
    /// Return error if the state construction failed.
    pub fn set_proxy(&self, proxy: Option<&Proxy>) -> Result<(), Error> {
        self.state.set_proxy(&self.email, proxy)
    }

    /// Check whether the account is logged in.
    ///
    /// An account is considered logged in if there is some value in the secret state.
    ///
    /// # Errors
    ///
    /// Returns error if the query failed.
    pub fn is_logged_out(&self) -> Result<bool, Error> {
        self.state.is_logged_out(&self.email)
    }

    /// Get the last poll event for this account.
    ///
    /// # Errors
    ///
    /// Returns error if the query failed.
    pub fn last_event(&self) -> Result<Option<Event>, Error> {
        self.state.last_event_for_account(&self.email)
    }

    fn from_row(row: &Row, state: &Arc<State>) -> rusqlite::Result<Self> {
        let email = row.get(0)?;
        let backend = row.get(1)?;
        let last_poll = row.get(2)?;
        Ok(Account::new(email, backend, last_poll, Arc::clone(state)))
    }
}

/// Trait that defines the behavior to be notified when the account list has changed/updated.
pub trait AccountWatcher: Send + Sync {
    /// Returns the new list of accounts.
    fn on_accounts_updated(&self, accounts: Vec<Account>);
}

struct AccountsUpdatedTableObserver<T: AccountWatcher> {
    state: Weak<State>,
    action: T,
}

impl<T: AccountWatcher> TableObserver for AccountsUpdatedTableObserver<T> {
    fn tables(&self) -> Vec<String> {
        vec!["yhm_poll_event".to_owned(), "yhm".to_owned()]
    }

    fn on_tables_changed(&self, _: &BTreeSet<String>) {
        if let Some(state) = self.state.upgrade() {
            match state.accounts() {
                Ok(accounts) => {
                    self.action.on_accounts_updated(accounts);
                }
                Err(e) => error!("Failed to get accounts on table updated: {e}"),
            }
        }
    }
}

/// Contains all state serialized in the database.
pub struct State {
    pool: Arc<RusqliteConnectionPool>,
    encryption_key: SecretBox<Key>,
}

impl State {
    /// Create a new state with database at `db_path` and with the given `encryption_key`.
    ///
    /// # Errors
    ///
    /// Returns errors if we failed to create the tables.
    pub fn new(
        db_path: PathBuf,
        encryption_key: SecretBox<Key>,
        watcher: Arc<Watcher>,
    ) -> Result<Arc<Self>, Error> {
        let pool = new_db_pool(db_path, watcher)?;
        let mut conn = pool.connection()?;
        conn.transaction_closure(create_tables)?;
        Ok(Arc::new(Self {
            pool,
            encryption_key,
        }))
    }

    /// Create a new state with database at `db_path` and with the given `encryption_key` without
    /// initializing the database tables.
    ///
    /// # Errors
    ///
    /// Returns errors if we failed to create the tables.
    pub fn without_init(
        db_path: PathBuf,
        encryption_key: SecretBox<Key>,
        watcher: Arc<Watcher>,
    ) -> Result<Arc<Self>, Error> {
        let pool = new_db_pool(db_path, watcher)?;
        Ok(Arc::new(Self {
            pool,
            encryption_key,
        }))
    }

    /// Get the encryption key.
    #[must_use]
    pub fn encryption_key(&self) -> &SecretBox<Key> {
        &self.encryption_key
    }

    /// Get database watcher instance.
    #[must_use]
    pub fn watcher(&self) -> &Arc<Watcher> {
        self.pool.watcher()
    }

    /// Get all accounts recorded in the database.
    ///
    /// # Errors
    ///
    /// Returns error if the query failed.
    pub fn accounts(self: &Arc<Self>) -> Result<Vec<Account>, Error> {
        let conn = self.pool.connection()?;
        let mut stmt = conn.prepare("SELECT email, backend, last_poll FROM yhm ORDER BY email")?;
        let rows = stmt.query_map((), |r| Account::from_row(r, self))?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    /// Register a watcher for the accounts table.
    ///
    /// # Errors
    ///
    /// Return error if we fail to register the watcher.
    pub fn watch_accounts<T: AccountWatcher + 'static>(
        self: &Arc<Self>,
        action: T,
    ) -> Result<DropRemoveTableObserverHandle, Error> {
        self.pool
            .watcher()
            .add_observer_with_drop_remove(Box::new(AccountsUpdatedTableObserver {
                state: Arc::downgrade(self),
                action,
            }))
            .map_err(|e| Error::Other(e.into()))
    }

    /// Get all accounts recorded in the database that are logged in.
    ///
    /// This returns any account which does not have their secret state set to NULL.
    ///
    /// # Errors
    ///
    /// Returns error if the query failed.
    pub fn active_accounts(self: &Arc<Self>) -> Result<Vec<Account>, Error> {
        let conn = self.pool.connection()?;
        let mut stmt =
            conn.prepare("SELECT email, backend, last_poll FROM yhm WHERE secret IS NOT NULL")?;
        let rows = stmt.query_map((), |r| Account::from_row(r, self))?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    /// Get a single account by `email`.
    ///
    /// # Errors
    ///
    /// Returns error if the query failed.
    pub fn account(self: &Arc<Self>, email: &str) -> Result<Option<Account>, Error> {
        let conn = self.pool.connection()?;
        Ok(conn
            .query_row(
                "SELECT email, backend, last_poll FROM yhm WHERE email=? LIMIT 1",
                [email],
                |r| Account::from_row(r, self),
            )
            .optional()?)
    }

    /// Get the number of registered accounts.
    ///
    /// # Errors
    ///
    /// Returns error if the query failed.
    pub fn account_count(&self) -> Result<usize, Error> {
        let conn = self.pool.connection()?;
        Ok(conn.query_row("SELECT count(*) FROM yhm", (), |r| r.get(0))?)
    }

    /// Check if account with `email` exists.
    ///
    /// # Errors
    ///
    /// Returns error if the query failed.
    pub fn has_account(&self, email: &str) -> Result<bool, Error> {
        let conn = self.pool.connection()?;
        Ok(conn
            .query_row("SELECT 1 FROM yhm WHERE email=? LIMIT 1", [email], |r| {
                r.get::<usize, i32>(0)
            })
            .optional()?
            .is_some())
    }

    /// Create new account with `email` and `backend`.
    ///
    /// # Errors
    ///
    /// Return error it the query failed.
    pub fn new_account(self: &Arc<Self>, email: &str, backend: &str) -> Result<Account, Error> {
        let mut conn = self.pool.connection()?;
        conn.transaction_closure(|tx| -> Result<(), rusqlite::Error> {
            tx.execute(
                r"
INSERT OR IGNORE INTO yhm (email, backend) VALUES (
    ?,?
)",
                (email, backend),
            )?;
            Ok(())
        })?;
        Ok(Account::new(
            email.to_owned(),
            backend.to_owned(),
            None,
            Arc::clone(self),
        ))
    }

    /// Delete account with `email`.
    ///
    /// # Errors
    ///
    /// Return error it the query failed.
    pub fn delete_account(&self, email: &str) -> Result<(), Error> {
        let mut conn = self.pool.connection()?;
        conn.transaction_closure(|tx| {
            tx.execute("DELETE FROM yhm WHERE email=?", [email])?;
            Ok(())
        })
    }

    /// Update `proxy` config for account with `email`.
    ///
    /// # Errors
    ///
    /// Returns error if the operation failed.
    pub fn set_proxy(&self, email: &str, proxy: Option<&Proxy>) -> Result<(), Error> {
        let bytes = match proxy {
            None => None,
            Some(proxy) => Some(secret_to_bytes(self.encryption_key.expose_secret(), proxy)?),
        };

        let mut conn = self.pool.connection()?;
        conn.transaction_closure(|tx| -> Result<(), Error> {
            tx.execute("UPDATE yhm SET proxy=? WHERE email=?", (bytes, email))?;
            Ok(())
        })
    }

    /// Get the proxy config of the account with `email`.
    ///
    /// # Errors
    ///
    /// Return error it the query failed.
    pub fn proxy(&self, email: &str) -> Result<Option<Proxy>, Error> {
        let conn = self.pool.connection()?;
        let proxy_bytes: Option<Vec<u8>> = conn.query_row(
            "SELECT proxy FROM yhm WHERE email=? LIMIT 1",
            [email],
            |r| r.get(0),
        )?;

        let proxy = match proxy_bytes {
            None => None,
            Some(proxy) => Some(secret_from_bytes::<Proxy>(
                self.encryption_key.expose_secret(),
                &proxy,
            )?),
        };

        Ok(proxy)
    }

    /// Update the `secret` state of account with `email`
    ///
    /// # Errors
    ///
    /// Return error it the query failed or the state failed to serialize.
    pub fn set_secret_state<T: Serialize>(&self, email: &str, secret: &T) -> Result<(), Error> {
        let bytes = secret_to_bytes(self.encryption_key.expose_secret(), secret)?;
        let mut conn = self.pool.connection()?;
        conn.transaction_closure(|tx| {
            tx.execute("UPDATE yhm SET secret=? WHERE email=?", (bytes, email))?;
            Ok(())
        })
    }

    /// Get the secret state of the account with `email`.
    ///
    /// # Errors
    ///
    /// Return error it the query failed.
    pub fn secret_state<T: DeserializeOwned>(&self, email: &str) -> Result<Option<T>, Error> {
        let conn = self.pool.connection()?;
        let secret_bytes: Option<Vec<u8>> = conn.query_row(
            "SELECT secret FROM yhm WHERE email=? LIMIT 1",
            [email],
            |r| r.get(0),
        )?;

        let secret = match secret_bytes {
            None => None,
            Some(secret) => Some(secret_from_bytes::<T>(
                self.encryption_key.expose_secret(),
                &secret,
            )?),
        };

        Ok(secret)
    }

    /// Remove the state of the account with `email`.
    ///
    /// # Errors
    ///
    /// Return error it the query failed.
    pub fn delete_account_state(&self, email: &str) -> Result<(), Error> {
        let mut conn = self.pool.connection()?;
        conn.transaction_closure(|tx| {
            tx.execute("UPDATE yhm SET state=NULL WHERE email=?", [email])?;
            Ok(())
        })
    }

    /// Update the `state` of account with `email`
    ///
    /// # Errors
    ///
    /// Return error it the query failed or the state failed to serialize.
    pub fn set_account_state<T: Serialize>(&self, email: &str, state: &T) -> Result<(), Error> {
        let bytes = serde_json::to_vec(state)?;
        let mut conn = self.pool.connection()?;
        conn.transaction_closure(|tx| {
            tx.execute("UPDATE yhm SET state=? WHERE email=?", (bytes, email))?;
            Ok(())
        })
    }

    /// Remove the secret state of the account with `email`.
    ///
    /// # Errors
    ///
    /// Return error it the query failed.
    pub fn delete_secret_state(&self, email: &str) -> Result<(), Error> {
        let mut conn = self.pool.connection()?;
        conn.transaction_closure(|tx| {
            tx.execute("UPDATE yhm SET secret=NULL WHERE email=?", [email])?;
            Ok(())
        })
    }

    /// Get the account state of the account with `email`.
    ///
    /// # Errors
    ///
    /// Return error it the query failed.
    pub fn account_state<T: DeserializeOwned>(&self, email: &str) -> Result<Option<T>, Error> {
        let conn = self.pool.connection()?;
        let state_bytes: Option<Vec<u8>> = conn.query_row(
            "SELECT state FROM yhm WHERE email=? LIMIT 1",
            [email],
            |r| r.get(0),
        )?;

        let state = match state_bytes {
            None => None,
            Some(state) => Some(serde_json::from_slice(&state)?),
        };

        Ok(state)
    }

    /// Delete account with `email`.
    ///
    /// # Errors
    ///
    /// Returns error if the operation failed.
    pub fn delete(&self, email: &str) -> Result<(), Error> {
        let mut conn = self.pool.connection()?;
        conn.transaction_closure(|tx| {
            tx.execute("DELETE FROM yhm WHERE email=?", [email])?;
            Ok(())
        })
    }

    /// Get poll interval setting.
    ///
    /// # Errors
    ///
    /// Return error if the operation failed.
    pub fn poll_interval(&self) -> Result<Duration, Error> {
        let conn = self.pool.connection()?;
        let interval: u64 = conn.query_row(
            "SELECT poll_interval FROM yhm_settings WHERE id=? LIMIT 1",
            [SETTINGS_ID],
            |r| r.get(0),
        )?;
        Ok(Duration::from_secs(interval))
    }

    /// Set the poll interval setting.
    ///
    /// # Errors
    ///
    /// Return error if the operation failed.
    pub fn set_poll_interval(&self, duration: Duration) -> Result<(), Error> {
        let mut conn = self.pool.connection()?;
        conn.transaction_closure(|tx| {
            tx.execute(
                "UPDATE yhm_settings SET poll_interval=? WHERE id=?",
                (duration.as_secs(), SETTINGS_ID),
            )?;
            Ok(())
        })
    }

    /// Check if account with `email` is logged out.
    ///
    /// # Errors
    ///
    /// Return error if the query failed.
    pub fn is_logged_out(&self, email: &str) -> Result<bool, Error> {
        let conn = self.pool.connection()?;
        Ok(conn.query_row(
            "SELECT IIF(secret IS NULL, 1, 0) FROM yhm WHERE email =?",
            [email],
            |r| r.get(0),
        )?)
    }

    /// Store `events` into the database
    ///
    /// # Errors
    ///
    /// Returns error if the process failed
    pub fn create_or_update_events(&self, events: &[Event]) -> Result<(), Error> {
        let mut conn = self.pool.connection()?;
        let time = Utc::now();
        conn.transaction_closure(|tx| {
            tx.execute("DELETE FROM yhm_poll_event", ())?;
            let mut event_stmt = tx.prepare(
                r"
WITH cte(email, event) AS (
    VALUES (?,?)
)
INSERT OR REPLACE INTO yhm_poll_event (email, event)
SELECT c.email,c.event FROM cte AS C
WHERE EXISTS (SELECT 1 FROM yhm WHERE email=c.email)
",
            )?;
            let mut update_account_stmt = tx.prepare("UPDATE yhm SET last_poll=? WHERE email=?")?;

            for event in events {
                let email = event.email();
                update_account_stmt.execute((time, email))?;
                event_stmt.execute((email, event))?;
            }

            Ok(())
        })
    }

    /// Load all events
    ///
    /// # Errors
    ///
    /// Returns error if the query failed.
    ///
    pub fn last_events(&self) -> Result<Vec<Event>, Error> {
        let conn = self.pool.connection()?;
        let mut stmt = conn.prepare("SELECT event FROM yhm_poll_event")?;
        let mut events = Vec::new();
        for row in stmt.query_map((), |r| r.get(0))? {
            events.push(row?);
        }
        Ok(events)
    }

    /// Get the last event result for the account with `email`.
    ///
    /// # Errors
    ///
    /// Returns error if the query fails.
    pub fn last_event_for_account(&self, email: &str) -> Result<Option<Event>, Error> {
        let conn = self.pool.connection()?;
        Ok(conn
            .query_row(
                "SELECT event FROM yhm_poll_event WHERE email=? LIMIT 1",
                [email],
                |r| r.get(0),
            )
            .optional()?)
    }

    /// Run a read only `closure` on the database.
    ///
    /// # Errors
    ///
    /// Returns error if the connection could not be acquired.
    pub fn db_read<T, E, F>(&self, closure: F) -> Result<T, E>
    where
        E: From<rusqlite::Error> + From<ConnectionPoolError<rusqlite::Error>>,
        F: FnOnce(&mut RusqlitePooledConnection) -> Result<T, E>,
    {
        let mut conn = self.pool.connection()?;
        closure(&mut conn)
    }

    /// Create a transaction and run the  `closure` on it.
    ///
    /// # Errors
    ///
    /// Returns error if the connection could not be acquired or the transaction could not be
    /// started.
    pub fn db_write<T, E, F>(&self, closure: F) -> Result<T, E>
    where
        E: From<rusqlite::Error> + From<ConnectionPoolError<rusqlite::Error>>,
        F: FnOnce(&mut RusqliteTransaction) -> Result<T, E>,
    {
        let mut conn = self.pool.connection()?;
        conn.transaction_closure(closure)
    }
}

fn create_tables(tx: &mut RusqliteTransaction) -> rusqlite::Result<()> {
    tx.execute(
        r"
CREATE TABLE IF NOT EXISTS yhm (
    email TEXT PRIMARY KEY,
    backend TEXT NOT NULL,
    secret BLOB DEFAULT NULL,
    state BLOD DEFAULT NULL,
    proxy BLOB DEFAULT NULL,
    last_poll INTEGER DEFAULT NULL
)
",
        (),
    )?;

    tx.execute(
        r"
CREATE TABLE IF NOT EXISTS yhm_settings (
    id PRIMARY KEY,
    poll_interval INTEGER NOT NULL DEFAULT 300
)
",
        (),
    )?;

    tx.execute(
        "INSERT OR IGNORE INTO yhm_settings VALUES (?,?)",
        (SETTINGS_ID, DEFAULT_POLL_INTERVAL_SECONDS),
    )?;

    tx.execute(
        r"
CREATE TABLE IF NOT EXISTS yhm_poll_event (
    email STRING NOT NULL UNIQUE,
    event STRING,
    FOREIGN KEY (email) REFERENCES yhm(email) ON DELETE CASCADE
)
",
        (),
    )?;

    Ok(())
}

/// Decrypted and deserialize secret.
///
/// # Errors
///
/// Returns error if the decryption or deserialization failed.
fn secret_from_bytes<T: DeserializeOwned>(key: &Key, bytes: &[u8]) -> Result<T, Error> {
    let decrypted = SecretSlice::new(key.decrypt(bytes)?.into());
    Ok(serde_json::from_slice::<T>(decrypted.expose_secret())?)
}

/// Serialize and encrypt secret.
///
/// # Errors
///
/// Returns error if the encryption or serialization failed.
fn secret_to_bytes<T: Serialize>(key: &Key, value: &T) -> Result<Vec<u8>, Error> {
    let serialized = SecretSlice::new(serde_json::to_vec(value)?.into());
    let encrypted = key.encrypt(serialized.expose_secret())?;
    Ok(encrypted)
}

const SETTINGS_ID: i64 = 1;
const DEFAULT_POLL_INTERVAL_SECONDS: i64 = 300;

fn new_db_pool(
    path: PathBuf,
    watcher: Arc<Watcher>,
) -> Result<Arc<RusqliteConnectionPool>, ConnectionPoolError<rusqlite::Error>> {
    let config = ConnectionPoolConfig {
        max_read_connection_count: 3,
        file_path: path,
        connection_acquire_timeout: None,
        watcher,
    };
    RusqliteConnectionPool::new(config)
}
