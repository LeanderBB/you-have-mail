use crate::backend::{AuthRefresher, Backend};
use crate::encryption::{decrypt, encrypt};
use crate::EncryptionKey;
use anyhow::anyhow;
use proton_api_rs::tokio;
use serde::{Deserialize, Serialize};
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
    #[error("An decryption occurred: {0}")]
    Decryption(#[source] anyhow::Error),
    #[error("A JSON deserialization error occurred: {0}")]
    JSON(#[source] anyhow::Error),
}

#[derive(Debug, Error)]
pub enum ConfigStoreError {
    #[error("An error occurred while serializing auth info account '{account}'")]
    BackendConfig {
        account: String,
        error: anyhow::Error,
    },
    #[error("An encryption occurred: {0}")]
    Encryption(#[source] anyhow::Error),
    #[error("A JSON serialization error occurred: {0}")]
    JSON(#[source] anyhow::Error),
}

impl Config {
    pub fn load(
        key: &EncryptionKey,
        backends: &[Box<dyn Backend>],
        data: &[u8],
    ) -> Result<Vec<Box<dyn AuthRefresher>>, ConfigLoadError> {
        let decrypted = decrypt(key, data).map_err(ConfigLoadError::Decryption)?;
        let config = serde_json::from_slice::<ConfigJSON>(decrypted.as_ref())
            .map_err(|e| ConfigLoadError::JSON(anyhow!(e)))?;

        let mut result = Vec::<Box<dyn AuthRefresher>>::with_capacity(config.accounts.len());

        fn find_backend_with_tag<'a>(
            backends: &'a [Box<dyn Backend>],
            tag: &str,
        ) -> Option<&'a dyn Backend> {
            for b in backends {
                if b.name() == tag {
                    return Some(b.as_ref());
                }
            }
            None
        }

        for account in config.accounts {
            let Some(b) = find_backend_with_tag(backends, &account.backend) else {
                return Err(ConfigLoadError::BackendNotFound {account:account.email,backend:account.backend });
            };

            let refresher = b.auth_refresher_from_config(account.value).map_err(|e| {
                ConfigLoadError::BackendConfig {
                    account: account.email,
                    backend: account.backend,
                    error: e,
                }
            })?;

            result.push(refresher);
        }

        Ok(result)
    }

    pub fn store<'a>(
        key: &EncryptionKey,
        accounts: impl Iterator<Item = &'a crate::Account>,
    ) -> Result<Box<[u8]>, ConfigStoreError> {
        let mut json_accounts = Vec::<ConfigJSONAccount>::new();

        for account in accounts {
            if !account.is_logged_in() {
                //TODO: Logged out account should still be stored!
                continue;
            }
            let account_impl = account.get_impl().unwrap();
            let (tag, value) = account_impl.auth_refresher_config().map_err(|e| {
                ConfigStoreError::BackendConfig {
                    account: account.email().to_string(),
                    error: anyhow!(e),
                }
            })?;

            json_accounts.push(ConfigJSONAccount {
                email: account.email().to_string(),
                backend: tag,
                value,
            })
        }

        let config_json = ConfigJSON {
            accounts: json_accounts,
        };

        let json =
            serde_json::to_vec(&config_json).map_err(|e| ConfigStoreError::JSON(anyhow!(e)))?;

        encrypt(key, &json).map_err(ConfigStoreError::Encryption)
    }
}

#[derive(Deserialize, Serialize)]
struct ConfigJSONAccount {
    email: String,
    backend: String,
    value: serde_json::Value,
}

#[derive(Deserialize, Serialize)]
struct ConfigJSON {
    accounts: Vec<ConfigJSONAccount>,
}

#[tokio::test]
async fn test_config_store_and_load() {
    let null_backed = crate::backend::null::new_backend(&[
        crate::backend::null::NullTestAccount {
            email: "foo".to_string(),
            password: "foo".to_string(),
            totp: None,
        },
        crate::backend::null::NullTestAccount {
            email: "bar".to_string(),
            password: "bar".to_string(),
            totp: None,
        },
    ]);

    let key = EncryptionKey::new();
    let account1 = null_backed.login("foo", "foo").await.unwrap();
    let account2 = null_backed.login("bar", "bar").await.unwrap();

    let config_encrypted = Config::store(&key, [account1, account2].iter()).unwrap();

    let accounts = Config::load(&key, &[null_backed], &config_encrypted).unwrap();

    assert_eq!(accounts.len(), 2);
    let mut logged_in_account = Vec::<crate::Account>::with_capacity(accounts.len());

    for a in accounts {
        logged_in_account.push(a.refresh().await.unwrap());
    }

    assert_eq!(logged_in_account[0].email(), "foo");
    assert_eq!(logged_in_account[1].email(), "bar");
}
