//! Null backend implementation, useful for testing.
use crate::backend::{
    Account, AuthRefresher, AwaitTotp, Backend, BackendError, BackendResult, NewEmailReply,
};
use crate::AccountState;
use anyhow::{anyhow, Error};
use async_trait::async_trait;
use proton_api_rs::tokio;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct NullTestAccount {
    pub email: String,
    pub password: String,
    pub totp: Option<String>,
    pub wait_time: Option<Duration>,
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
}

#[doc(hidden)]
#[derive(Debug)]
struct NullAwaitTotp {
    email: String,
    totp: String,
    wait_time: Option<Duration>,
}

#[doc(hidden)]
#[derive(Debug)]
struct NullAuthRefresher {
    email: String,
}

const NULL_BACKEND_NAME: &str = "Null Backend";

#[async_trait]
impl Backend for NullBacked {
    fn name(&self) -> &str {
        NULL_BACKEND_NAME
    }

    fn description(&self) -> &str {
        "Test backend to verify app behavior"
    }

    async fn login(&self, email: &str, password: &str) -> BackendResult<AccountState> {
        if let Some(account) = self.accounts.get(email) {
            if let Some(d) = account.wait_time {
                tokio::time::sleep(d).await;
            }

            if account.password != password {
                return Err(BackendError::Request(anyhow!(
                    "invalid user name or password"
                )));
            }

            return if let Some(totp) = &account.totp {
                Ok(AccountState::AwaitingTotp(Box::new(NullAwaitTotp {
                    email: email.to_string(),
                    totp: totp.clone(),
                    wait_time: account.wait_time,
                })))
            } else {
                Ok(AccountState::LoggedIn(Box::new(NullAccount {
                    email: email.to_string(),
                    wait_time: account.wait_time,
                })))
            };
        }

        return Err(BackendError::Request(anyhow!(
            "invalid user name or password"
        )));
    }

    fn auth_refresher_from_config(&self, value: Value) -> Result<Box<dyn AuthRefresher>, Error> {
        let cfg = serde_json::from_value::<NullAuthRefresherInfo>(value).map_err(|e| anyhow!(e))?;
        Ok(Box::new(NullAuthRefresher { email: cfg.email }))
    }
}

#[async_trait]
impl AuthRefresher for NullAuthRefresher {
    async fn refresh(self: Box<Self>) -> Result<AccountState, BackendError> {
        Ok(AccountState::LoggedIn(Box::new(NullAccount {
            email: self.email,
            wait_time: None,
        })))
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct NullAuthRefresherInfo {
    email: String,
}

#[async_trait]
impl Account for NullAccount {
    async fn check(&mut self) -> BackendResult<NewEmailReply> {
        Ok(NewEmailReply { count: 1 })
    }

    async fn logout(&mut self) -> BackendResult<()> {
        if let Some(d) = self.wait_time {
            tokio::time::sleep(d).await;
        }
        Ok(())
    }

    fn auth_refresher_config(&self) -> Result<Value, Error> {
        serde_json::to_value(NullAuthRefresherInfo {
            email: self.email.clone(),
        })
        .map_err(|e| anyhow!(e))
    }
}

#[async_trait]
impl AwaitTotp for NullAwaitTotp {
    async fn submit_totp(
        self: Box<NullAwaitTotp>,
        totp: &str,
    ) -> Result<Box<dyn Account>, (Box<dyn AwaitTotp>, BackendError)> {
        if let Some(d) = self.wait_time {
            tokio::time::sleep(d).await;
        }

        if self.totp != totp {
            return Err((self, BackendError::Request(anyhow!("Invalid totp"))));
        }

        Ok(Box::new(NullAccount {
            email: self.email,
            wait_time: self.wait_time,
        }))
    }
}
