use crate::backend::{AuthRefresher, Backend};
use crate::Account;
use anyhow::anyhow;
use proton_api_rs::log::error;
use proton_api_rs::tokio;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

/// Config stores a You Have Mail application state with all the active user accounts
/// and their auth tokens.
#[derive(Copy, Clone)]
pub struct Config {}

#[derive(Debug, Error)]
pub enum ConfigLoadError {
    #[error("Backend '{backend}' for account '{account}' was not found")]
    BackendNotFound { account: String, backend: String },
    #[error(
        "An error occurred while deserializing auth info for '{backend}' with account '{account}'"
    )]
    BackendConfig {
        account: String,
        backend: String,
        error: anyhow::Error,
    },
    #[error("A JSON deserialization error occurred: {0}")]
    JSON(#[source] anyhow::Error),
}

#[derive(Debug, Error)]
pub enum ConfigGenError {
    #[error("An error occurred while serializing auth info account '{account}'")]
    BackendConfig {
        account: String,
        error: anyhow::Error,
    },
    #[error("A JSON serialization error occurred: {0}")]
    JSON(#[source] anyhow::Error),
}

pub type ConfigAccount = (Account, Option<Box<dyn AuthRefresher>>);

impl Config {
    pub fn load(
        backends: &[Arc<dyn Backend>],
        data: &[u8],
    ) -> Result<(Duration, Vec<ConfigAccount>), ConfigLoadError> {
        let config = serde_json::from_slice::<ConfigJSON>(data)
            .map_err(|e| ConfigLoadError::JSON(anyhow!(e)))?;

        let mut result = Vec::with_capacity(config.accounts.len());

        fn find_backend_with_tag(
            backends: &[Arc<dyn Backend>],
            tag: &str,
        ) -> Option<Arc<dyn Backend>> {
            for b in backends {
                if b.name() == tag {
                    return Some(b.clone());
                }
            }
            None
        }

        for account in config.accounts {
            let Some(b) = find_backend_with_tag(backends, &account.backend) else {
                error!("Could not locate backend '{}' for account '{}' skipping...", account.backend, account.email);
                continue
            };

            let refresher = if let Some(value) = account.value {
                Some(b.auth_refresher_from_config(value).map_err(|e| {
                    ConfigLoadError::BackendConfig {
                        account: account.email.clone(),
                        backend: account.backend,
                        error: e,
                    }
                })?)
            } else {
                None
            };

            let account = Account::new(b, account.email);

            result.push((account, refresher));
        }

        Ok((
            Duration::from_secs(config.poll_interval.unwrap_or(5 * 60)),
            result,
        ))
    }

    pub fn store<'a>(
        poll_interval: Duration,
        accounts: impl Iterator<Item = &'a Account>,
    ) -> Result<String, ConfigGenError> {
        let mut json_accounts = Vec::<ConfigJSONAccount>::new();

        for account in accounts {
            let value = if !account.is_logged_out() {
                let account_impl = account.get_impl().unwrap();
                Some(account_impl.auth_refresher_config().map_err(|e| {
                    ConfigGenError::BackendConfig {
                        account: account.email().to_string(),
                        error: anyhow!(e),
                    }
                })?)
            } else {
                None
            };

            json_accounts.push(ConfigJSONAccount {
                email: account.email().to_string(),
                backend: account.backend().name().to_string(),
                value,
            })
        }

        let config_json = ConfigJSON {
            poll_interval: Some(poll_interval.as_secs()),
            accounts: json_accounts,
        };

        serde_json::to_string(&config_json).map_err(|e| ConfigGenError::JSON(anyhow!(e)))
    }
}

#[derive(Deserialize, Serialize)]
struct ConfigJSONAccount {
    email: String,
    backend: String,
    value: Option<serde_json::Value>,
}

#[derive(Deserialize, Serialize)]
struct ConfigJSON {
    poll_interval: Option<u64>,
    accounts: Vec<ConfigJSONAccount>,
}

#[tokio::test]
async fn test_config_store_and_load() {
    let null_backed = crate::backend::null::new_backend(&[
        crate::backend::null::NullTestAccount {
            email: "foo".to_string(),
            password: "foo".to_string(),
            totp: None,
            wait_time: None,
        },
        crate::backend::null::NullTestAccount {
            email: "bar".to_string(),
            password: "bar".to_string(),
            totp: None,
            wait_time: None,
        },
    ]);

    let poll_interval = Duration::from_secs(10);

    let account1 = {
        let mut a = Account::new(null_backed.clone(), "foo");
        a.login("foo").await.unwrap();
        a
    };
    let account2 = Account::new(null_backed.clone(), "bar");

    let config_generated = Config::store(poll_interval, [account1, account2].iter()).unwrap();

    let (loaded_poll_interval, accounts) =
        Config::load(&[null_backed], config_generated.as_bytes()).unwrap();

    assert_eq!(accounts.len(), 2);
    assert_eq!(accounts[0].0.email(), "foo");
    assert_eq!(accounts[1].0.email(), "bar");
    assert!(accounts[0].1.is_some());
    assert!(accounts[1].1.is_none());
    assert_eq!(poll_interval, loaded_poll_interval);
}
