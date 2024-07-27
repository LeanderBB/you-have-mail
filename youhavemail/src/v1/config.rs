use crate::encryption::Key;
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;
use tracing::debug;

#[derive(Debug, Error)]
pub enum Error {
    #[error("IO error occurred: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error occurred:{0}")]
    Json(#[from] serde_json::Error),
    #[error("Crypto error occurred: {0}")]
    State(#[from] crate::state::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
struct Account {
    email: String,
    backend: String,
    value: Option<serde_json::Value>,
    proxy: Option<Proxy>,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct Config {
    pub poll_interval: Option<u64>,
    accounts: Vec<Account>,
}

impl Config {
    /// Convert existing information to v2 accounts.
    pub fn to_v2_accounts(&self, encryption_key: &Key) -> Result<Vec<crate::state::Account>> {
        let mut result = Vec::with_capacity(self.accounts.len());
        for account in &self.accounts {
            let mut v2 = crate::state::Account::new(
                account.email.clone(),
                // there were no other accounts in v1.
                crate::backend::proton::NAME.to_owned(),
            );
            let proxy = account.proxy.clone().map(|v| http::Proxy {
                protocol: match v.protocol {
                    ProxyProtocol::Https => http::ProxyProtocol::Https,
                    ProxyProtocol::Socks5 => http::ProxyProtocol::Socks5,
                },
                auth: v.auth.map(|auth| http::ProxyAuth {
                    username: auth.username,
                    password: SecretString::new(auth.password),
                }),
                host: v.url,
                port: v.port,
            });
            v2.set_proxy(encryption_key, proxy.as_ref())?;
            result.push(v2);
        }

        Ok(result)
    }
}

/// Load v1 config file.
pub fn load(encryption_key: &Key, file_path: &Path) -> Result<Config> {
    match std::fs::read(file_path) {
        Err(e) => {
            if e.kind() != std::io::ErrorKind::NotFound {
                return Err(e.into());
            }

            debug!("v1 config not found");
            Ok(Config {
                poll_interval: None,
                accounts: vec![],
            })
        }
        Ok(data) => {
            let decrypted = encryption_key
                .decrypt(&data)
                .map_err(crate::state::Error::from)?;
            Ok(serde_json::from_slice::<Config>(&decrypted)?)
        }
    }
}
#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
enum ProxyProtocol {
    Https,
    Socks5,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
struct ProxyAuth {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
struct Proxy {
    pub protocol: ProxyProtocol,
    pub auth: Option<ProxyAuth>,
    pub url: String,
    pub port: u16,
}

#[test]
fn test_config_v1_into_v2() {
    use secrecy::ExposeSecret;
    use std::time::Duration;
    let tmp_dir = temp_dir::TempDir::new().expect("failed to create tmp dir");
    let encryption_key = Key::new();
    let config_path = tmp_dir.child("config");

    let poll_interval = Duration::from_secs(10);
    let proxy = Proxy {
        protocol: ProxyProtocol::Socks5,
        auth: Some(ProxyAuth {
            username: "Hello".into(),
            password: "Goodbye".into(),
        }),
        url: "127.0.0.1".into(),
        port: 1080,
    };

    let config = Config {
        poll_interval: Some(poll_interval.as_secs()),
        accounts: vec![
            Account {
                email: "foo".to_string(),
                backend: "foo".to_string(),
                value: None,
                proxy: Some(proxy.clone()),
            },
            Account {
                email: "bar".to_string(),
                backend: "bar".to_string(),
                value: None,
                proxy: None,
            },
        ],
    };

    let encrypted = encryption_key
        .expose_secret()
        .encrypt(serde_json::to_vec(&config).unwrap().as_ref())
        .unwrap();
    std::fs::write(&config_path, encrypted).unwrap();

    let config_loaded = load(encryption_key.expose_secret(), &config_path).unwrap();
    assert_eq!(config_loaded, config);

    let accounts_v2 = config_loaded
        .to_v2_accounts(encryption_key.expose_secret())
        .unwrap();

    assert_eq!(accounts_v2[0].email(), config_loaded.accounts[0].email);
    assert_eq!(accounts_v2[0].backend(), crate::backend::proton::NAME);
    assert!(accounts_v2[0].is_logged_out());
    assert!(accounts_v2[0]
        .proxy(encryption_key.expose_secret())
        .unwrap()
        .is_some());

    assert_eq!(accounts_v2[1].email(), config_loaded.accounts[1].email);
    assert_eq!(accounts_v2[1].backend(), crate::backend::proton::NAME);
    assert!(accounts_v2[1].is_logged_out());
    assert!(accounts_v2[1]
        .proxy(encryption_key.expose_secret())
        .unwrap()
        .is_none());
}
