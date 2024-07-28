//! Dummy backend which always produces an email notification. Is mostly used for testing.

use crate::backend::NewEmail;
use crate::state::Account;
use crate::yhm::Yhm;
use http::{Client, Proxy};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub const NAME: &str = "Dummy";

/// This backend always produces a notification for each account.
pub struct Backend {}

impl Backend {
    /// Create a new instance
    #[must_use]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {})
    }

    /// Create new dummy account
    ///
    /// # Errors
    ///
    /// Returns error if the process failed.
    pub fn new_dummy_account(yhm: &Yhm) -> Result<(), crate::yhm::Error> {
        let account = yhm.new_account(DUMMY_EMAIL, NAME)?;
        let secret = Auth {};
        account.set_secret(Some(&secret))?;
        Ok(())
    }
}

impl crate::backend::Backend for Backend {
    fn name(&self) -> &str {
        NAME
    }

    fn description(&self) -> &str {
        "Testing Backed"
    }

    fn create_client(&self, proxy: Option<Proxy>) -> crate::backend::Result<Arc<Client>> {
        let mut builder =
            Client::builder(http::url::Url::parse("127.0.0.1:8080").unwrap()).allow_http();
        if let Some(proxy) = proxy {
            builder = builder.with_proxy(proxy);
        }
        Ok(builder.build()?)
    }

    fn new_poller(
        &self,
        _: Arc<Client>,
        account: Account,
    ) -> crate::backend::Result<Box<dyn crate::backend::Poller>> {
        Ok(Box::new(Poller(account)))
    }
}

struct Poller(Account);

#[derive(Serialize, Deserialize)]
struct Auth {}

impl crate::backend::Poller for Poller {
    fn check(&mut self) -> crate::backend::Result<Vec<NewEmail>> {
        Ok(vec![NewEmail {
            sender: DUMMY_EMAIL.to_owned(),
            subject: "You Have Mail".to_owned(),
        }])
    }

    fn logout(&mut self) -> crate::backend::Result<()> {
        self.0.set_secret::<Auth>(None)?;
        Ok(())
    }
}

const DUMMY_EMAIL: &str = "dummy@dummy.net";
