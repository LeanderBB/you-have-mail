//! Null backend implementation, useful for testing.
use crate::backend::{Account, AwaitTotp, Backend, BackendError, BackendResult, NewEmailReply};
use crate::AccountState;
use anyhow::anyhow;
use async_trait::async_trait;
use std::collections::HashMap;

#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct NullTestAccount {
    pub email: String,
    pub password: String,
    pub totp: String,
}

pub fn new_null_backend(accounts: &[NullTestAccount]) -> Box<dyn Backend> {
    Box::new(NullBacked {
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
struct NullAccount {}

#[doc(hidden)]
#[derive(Debug)]
struct NullAwaitTotp {
    totp: String,
}

#[async_trait]
impl Backend for NullBacked {
    fn name(&self) -> &str {
        "null backend"
    }

    async fn login(&self, username: &str, password: &str) -> BackendResult<AccountState> {
        if let Some(account) = self.accounts.get(username) {
            if account.password != password {
                return Err(BackendError::Request(anyhow!(
                    "invalid user name or password"
                )));
            }

            return if !account.totp.is_empty() {
                Ok(AccountState::AwaitingTotp(Box::new(NullAwaitTotp {
                    totp: account.totp.clone(),
                })))
            } else {
                Ok(AccountState::LoggedIn(Box::new(NullAccount {})))
            };
        }

        return Err(BackendError::Request(anyhow!(
            "invalid user name or password"
        )));
    }
}

#[async_trait]
impl Account for NullAccount {
    async fn check(&mut self) -> BackendResult<NewEmailReply> {
        Ok(NewEmailReply { count: 1 })
    }

    async fn logout(&mut self) -> BackendResult<()> {
        Ok(())
    }
}

#[async_trait]
impl AwaitTotp for NullAwaitTotp {
    async fn submit_totp(
        self: Box<NullAwaitTotp>,
        totp: &str,
    ) -> Result<Box<dyn Account>, (Box<dyn AwaitTotp>, BackendError)> {
        if self.totp != totp {
            return Err((self, BackendError::Request(anyhow!("Invalid totp"))));
        }

        Ok(Box::new(NullAccount {}))
    }
}
