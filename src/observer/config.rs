use crate::{decrypt, encrypt, Account, EncryptionKey, Proxy};
use parking_lot::RwLock;
use proton_api_rs::log::debug;
use secrecy::Secret;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("IO error occurred: {0}")]
    IO(#[from] std::io::Error),
    #[error("Serialization error occurred:{0}")]
    JSON(#[from] serde_json::Error),
    #[error("Crypto error occurred: {0}")]
    Crypto(#[source] anyhow::Error),
    #[error("Unknown error occurred: {0}")]
    Unknown(
        #[from]
        #[source]
        anyhow::Error,
    ),
}

pub type ConfigResult<T> = Result<T, ConfigError>;

#[derive(Clone)]
pub struct Config(Arc<RwLock<ConfigInner>>);

pub enum ConfigAuthRefresher {
    Resolved(serde_json::Value),
    None,
}

pub(crate) struct ConfigAccount {
    pub backend: String,
    pub auth_refresher: ConfigAuthRefresher,
    pub proxy: Option<Proxy>,
}

pub(crate) struct ConfigInner {
    file_path: PathBuf,
    encryption_key: Secret<EncryptionKey>,
    poll_interval: Duration,
    accounts: HashMap<String, ConfigAccount>,
    dirty: bool,
}

impl ConfigInner {
    pub fn add_or_update_account(&mut self, account: &Account) -> Result<(), anyhow::Error> {
        let refresh_data = if let Some(a) = account.get_impl() {
            ConfigAuthRefresher::Resolved(a.to_config()?)
        } else {
            ConfigAuthRefresher::None
        };

        self.accounts.insert(
            account.email().into(),
            ConfigAccount {
                backend: account.backend().name().into(),
                auth_refresher: refresh_data,
                proxy: account.get_proxy().clone(),
            },
        );

        self.dirty = true;

        Ok(())
    }

    pub fn account_removed(&mut self, email: &str) {
        if self.accounts.remove(email).is_some() {
            self.dirty = true;
        }
    }

    pub fn account_refreshed(&mut self, email: &str, value: serde_json::Value) {
        if let Some(account) = self.accounts.get_mut(email) {
            account.auth_refresher = ConfigAuthRefresher::Resolved(value);
            self.dirty = true;
        }
    }

    pub fn account_logged_out(&mut self, email: &str) {
        if let Some(account) = self.accounts.get_mut(email) {
            account.auth_refresher = ConfigAuthRefresher::None;
            self.dirty = true;
        }
    }

    pub fn account_proxy_changed(&mut self, email: &str, proxy: Option<Proxy>) {
        if let Some(account) = self.accounts.get_mut(email) {
            account.proxy = proxy;
            self.dirty = true;
        }
    }

    #[allow(unused)]
    pub fn get_account(&self, email: &str) -> Option<&ConfigAccount> {
        self.accounts.get(email)
    }

    pub fn get_accounts(&self) -> impl Iterator<Item = (&String, &ConfigAccount)> {
        self.accounts.iter()
    }

    pub fn len(&self) -> usize {
        self.accounts.len()
    }

    #[allow(unused)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn set_poll_interval(&mut self, interval: Duration) {
        if interval != self.poll_interval {
            self.poll_interval = interval;
            self.dirty = true;
        }
    }

    pub fn poll_interval(&self) -> Duration {
        self.poll_interval
    }
}

impl Config {
    pub fn new(
        encryption_key: Secret<EncryptionKey>,
        file_path: impl Into<PathBuf>,
        poll_interval: Duration,
    ) -> ConfigResult<Self> {
        let config = Self(Arc::new(RwLock::new(ConfigInner {
            file_path: file_path.into(),
            encryption_key,
            poll_interval,
            accounts: Default::default(),
            dirty: true,
        })));
        config.write(|_| Ok(()))?;
        Ok(config)
    }

    pub fn create_or_load(
        encryption_key: Secret<EncryptionKey>,
        file_path: impl Into<PathBuf>,
    ) -> ConfigResult<Self> {
        let file_path = file_path.into();

        let config_inner = match std::fs::read(&file_path) {
            Err(e) => {
                if e.kind() != std::io::ErrorKind::NotFound {
                    return Err(e.into());
                }

                ConfigInner {
                    file_path,
                    encryption_key,
                    poll_interval: Duration::from_secs(300),
                    accounts: Default::default(),
                    dirty: false,
                }
            }
            Ok(data) => {
                let decrypted = decrypt(&encryption_key, &data).map_err(ConfigError::Crypto)?;
                let config = serde_json::from_slice::<ConfigJSONRead>(&decrypted)?;

                let mut accounts = HashMap::with_capacity(config.accounts.len());
                for account in config.accounts {
                    accounts.insert(
                        account.email,
                        ConfigAccount {
                            backend: account.backend,
                            auth_refresher: if let Some(v) = account.value {
                                ConfigAuthRefresher::Resolved(v)
                            } else {
                                ConfigAuthRefresher::None
                            },
                            proxy: account.proxy,
                        },
                    );
                }

                ConfigInner {
                    file_path,
                    encryption_key,
                    poll_interval: Duration::from_secs(config.poll_interval.unwrap_or(300)),
                    accounts,
                    dirty: false,
                }
            }
        };

        Ok(Self(Arc::new(RwLock::new(config_inner))))
    }

    pub(crate) fn read<R>(&self, f: impl FnOnce(&ConfigInner) -> R) -> R {
        let accessor = self.0.read();
        (f)(accessor.deref())
    }

    pub(crate) fn write<R>(
        &self,
        f: impl FnOnce(&mut ConfigInner) -> Result<R, anyhow::Error>,
    ) -> ConfigResult<R> {
        let mut accessor = self.0.write();
        let r = (f)(accessor.deref_mut()).map_err(ConfigError::Unknown)?;

        if accessor.dirty {
            debug!("Config is dirty, writing to disk");
            Self::store(accessor.deref())?;
            accessor.dirty = false;
        }

        Ok(r)
    }

    fn store(config: &ConfigInner) -> ConfigResult<()> {
        debug!("Generating config json");
        let mut json_accounts = Vec::with_capacity(config.accounts.len());
        for (k, v) in &config.accounts {
            let refresher_value = match &v.auth_refresher {
                ConfigAuthRefresher::Resolved(v) => Some(v.clone()),
                ConfigAuthRefresher::None => None,
            };

            json_accounts.push(ConfigJSONAccount {
                email: k.clone(),
                backend: v.backend.clone(),
                value: refresher_value,
                proxy: v.proxy.clone(),
            });
        }

        let json_data = ConfigJSONWrite {
            poll_interval: Some(config.poll_interval.as_secs()),
            accounts: &json_accounts,
        };

        let json = serde_json::to_vec(&json_data)?;
        debug!("Encrypting config");
        let encrypted = encrypt(&config.encryption_key, &json).map_err(ConfigError::Crypto)?;

        let tmp_file = config.file_path.with_extension(".new");
        debug!("Writing tmp file");
        std::fs::write(&tmp_file, encrypted)?;
        debug!("Overwriting original file");
        std::fs::rename(&tmp_file, &config.file_path)?;
        Ok(())
    }
}

#[derive(Deserialize, Serialize)]
struct ConfigJSONAccount {
    email: String,
    backend: String,
    value: Option<serde_json::Value>,
    proxy: Option<Proxy>,
}

#[derive(Serialize)]
struct ConfigJSONWrite<'a> {
    poll_interval: Option<u64>,
    accounts: &'a [ConfigJSONAccount],
}

#[derive(Deserialize)]
struct ConfigJSONRead {
    poll_interval: Option<u64>,
    accounts: Vec<ConfigJSONAccount>,
}

#[test]
fn test_config_v2_store_and_load() {
    use crate::{ProxyAuth, ProxyProtocol};
    use secrecy::SecretString;

    let tmp_dir = temp_dir::TempDir::new().expect("failed to create tmp dir");
    let encryption_key = EncryptionKey::new();
    let config_path = tmp_dir.child("config");

    let proxy = Proxy {
        protocol: ProxyProtocol::Socks5,
        auth: Some(ProxyAuth {
            username: "Hello".into(),
            password: "Goodbye".into(),
        }),
        url: "127.0.0.1".into(),
        port: 1080,
    };

    let null_backed = crate::backend::null::new_backend(&[
        crate::backend::null::NullTestAccount {
            email: "foo".to_string(),
            password: "foo".to_string(),
            totp: None,
            wait_time: None,
            refresh: false,
        },
        crate::backend::null::NullTestAccount {
            email: "bar".to_string(),
            password: "bar".to_string(),
            totp: None,
            wait_time: None,
            refresh: false,
        },
    ]);

    let poll_interval = Duration::from_secs(10);

    let account1 = {
        let mut a = Account::new(null_backed.clone(), "foo", Some(proxy.clone()));
        a.login(&SecretString::new("foo".into()), None).unwrap();
        a
    };
    let account2 = Account::new(null_backed.clone(), "bar", None);

    // Initialize config
    {
        let config = Config::create_or_load(encryption_key.clone(), config_path.clone())
            .expect("Failed to init config");

        config.read(|inner| {
            assert_eq!(inner.len(), 0);
        });

        config
            .write(|inner| {
                inner.add_or_update_account(&account1)?;
                inner.add_or_update_account(&account2)?;
                inner.set_poll_interval(poll_interval);
                Ok(())
            })
            .expect("failed to update config");
    }

    // Load config second time
    {
        let config = Config::create_or_load(encryption_key.clone(), config_path.clone())
            .expect("Failed to init config");

        config
            .write(|inner| {
                assert_eq!(inner.len(), 2);

                {
                    let account = inner.get_account("foo").expect("Account not found");
                    assert_eq!(account.backend, null_backed.name());
                    assert_eq!(account.proxy, Some(proxy));
                    assert!(matches!(
                        account.auth_refresher,
                        ConfigAuthRefresher::Resolved(_)
                    ));
                }
                {
                    let account = inner.get_account("bar").expect("Account not found");
                    assert_eq!(account.backend, null_backed.name());
                    assert_eq!(account.proxy, None);
                    assert!(matches!(account.auth_refresher, ConfigAuthRefresher::None));
                }
                assert_eq!(inner.poll_interval(), poll_interval);

                inner.account_logged_out("foo");
                inner.account_removed("bar");
                Ok(())
            })
            .expect("failed to update config");
    }

    // Load config a second time and check that the changes were applied.
    {
        let config = Config::create_or_load(encryption_key.clone(), config_path.clone())
            .expect("Failed to init config");

        config.read(|inner| {
            assert_eq!(inner.len(), 1);
            let account = inner.get_account("foo").expect("Account not found");
            assert_eq!(account.backend, null_backed.name());
            assert!(account.proxy.is_some());
            assert!(matches!(account.auth_refresher, ConfigAuthRefresher::None));
        });
    }
}
