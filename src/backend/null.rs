//! Null backend implementation, useful for testing.
use crate::backend::{
    Account, AccountRefreshedNotifier, AwaitTotp, Backend, BackendError, BackendResult, CheckTask,
    EmailInfo, NewEmailReply,
};
use crate::{AccountState, Proxy};
use anyhow::{anyhow, Error};
use proton_api_rs::domain::SecretString;
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct NullTestAccount {
    pub email: String,
    pub password: String,
    pub totp: Option<String>,
    pub wait_time: Option<Duration>,
    pub refresh: bool,
}

#[doc(hidden)]
pub fn new_backend(accounts: &[NullTestAccount]) -> Arc<dyn Backend> {
    Arc::new(NullBacked {
        accounts: HashMap::from_iter(accounts.iter().map(|a| (a.email.clone(), a.clone()))),
    })
}

#[doc(hidden)]
#[derive(Debug)]
struct NullBacked {
    accounts: HashMap<String, NullTestAccount>,
}

#[doc(hidden)]
#[derive(Debug)]
struct NullAccount {
    email: String,
    wait_time: Option<Duration>,
    counter: Arc<AtomicUsize>,
    refresh: bool,
}

#[doc(hidden)]
#[derive(Debug)]
struct NullAwaitTotp {
    email: String,
    totp: String,
    wait_time: Option<Duration>,
    refresh: bool,
}

const NULL_BACKEND_NAME: &str = "Null Backend";

impl Backend for NullBacked {
    fn name(&self) -> &str {
        NULL_BACKEND_NAME
    }

    fn description(&self) -> &str {
        "Test backend to verify app behavior"
    }

    fn login(
        &self,
        username: &str,
        password: &SecretString,
        _: Option<&Proxy>,
        _: Option<String>,
    ) -> BackendResult<AccountState> {
        if let Some(account) = self.accounts.get(username) {
            if let Some(d) = account.wait_time {
                std::thread::sleep(d);
            }

            if account.password.as_str() != password.expose_secret() {
                return Err(BackendError::Request(anyhow!(
                    "invalid user name or password"
                )));
            }

            return if let Some(totp) = &account.totp {
                Ok(AccountState::AwaitingTotp(Box::new(NullAwaitTotp {
                    email: username.to_string(),
                    totp: totp.clone(),
                    wait_time: account.wait_time,
                    refresh: account.refresh,
                })))
            } else {
                Ok(AccountState::LoggedIn(Box::new(NullAccount {
                    email: username.to_string(),
                    wait_time: account.wait_time,
                    counter: Arc::new(AtomicUsize::new(0)),
                    refresh: account.refresh,
                })))
            };
        }

        Err(BackendError::Request(anyhow!(
            "invalid user name or password"
        )))
    }

    fn check_proxy(&self, _: &Proxy) -> BackendResult<()> {
        Ok(())
    }

    fn account_from_config(&self, _: Option<&Proxy>, value: Value) -> Result<AccountState, Error> {
        let cfg = serde_json::from_value::<NullAuthRefresherInfo>(value).map_err(|e| anyhow!(e))?;
        Ok(AccountState::LoggedIn(Box::new(NullAccount {
            email: cfg.email,
            wait_time: None,
            counter: Arc::new(AtomicUsize::new(cfg.counter)),
            refresh: false,
        })))
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct NullAuthRefresherInfo {
    email: String,
    counter: usize,
}

impl NullAccount {
    pub(crate) fn create_email_info(counter: usize) -> EmailInfo {
        EmailInfo {
            sender: format!("Null {}", counter),
            subject: format!("Null Subject {}", counter),
        }
    }

    pub(crate) fn create_email_reply(counter: usize) -> NewEmailReply {
        NewEmailReply {
            emails: vec![Self::create_email_info(counter)],
        }
    }
}

#[derive(Debug)]
struct NullTask {
    email: String,
    counter: Arc<AtomicUsize>,
    refresh: bool,
}

impl CheckTask for NullTask {
    fn email(&self) -> &str {
        &self.email
    }

    fn backend_name(&self) -> &str {
        NULL_BACKEND_NAME
    }

    fn check(&self, r: &mut dyn AccountRefreshedNotifier) -> BackendResult<NewEmailReply> {
        let val = self.counter.fetch_add(1, Ordering::SeqCst) + 1;
        if self.refresh {
            let cfg = self.to_config().unwrap();
            r.notify_account_refreshed(self.email(), cfg);
        }
        Ok(NullAccount::create_email_reply(val))
    }

    fn to_config(&self) -> Result<Value, Error> {
        serde_json::to_value(NullAuthRefresherInfo {
            email: self.email.clone(),
            counter: self.counter.load(Ordering::SeqCst),
        })
        .map_err(|e| anyhow!(e))
    }
}

impl Account for NullAccount {
    fn new_task(&self) -> Box<dyn CheckTask> {
        Box::new(NullTask {
            email: self.email.clone(),
            counter: self.counter.clone(),
            refresh: self.refresh,
        })
    }

    fn logout(&mut self) -> BackendResult<()> {
        if let Some(d) = self.wait_time {
            std::thread::sleep(d);
        }
        Ok(())
    }

    fn set_proxy(&mut self, _: Option<&Proxy>) -> BackendResult<()> {
        Ok(())
    }

    fn to_config(&self) -> Result<Value, Error> {
        serde_json::to_value(NullAuthRefresherInfo {
            email: self.email.clone(),
            counter: self.counter.load(Ordering::SeqCst),
        })
        .map_err(|e| anyhow!(e))
    }
}

impl AwaitTotp for NullAwaitTotp {
    fn submit_totp(&self, totp: &str) -> Result<Box<dyn Account>, BackendError> {
        if let Some(d) = self.wait_time {
            std::thread::sleep(d);
        }

        if self.totp != totp {
            return Err(BackendError::Request(anyhow!("Invalid totp")));
        }

        Ok(Box::new(NullAccount {
            email: self.email.clone(),
            wait_time: self.wait_time,
            counter: Arc::new(AtomicUsize::new(0)),
            refresh: self.refresh,
        }))
    }
}
