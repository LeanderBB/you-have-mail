//! Dummy backend which always produces an email notification. Is mostly used for testing.

use crate::backend::NewEmail;
use crate::state::Account;
use http::{Client, Proxy};
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
        _: &Account,
    ) -> crate::backend::Result<Box<dyn crate::backend::Poller>> {
        Ok(Box::new(Poller {}))
    }
}

struct Poller {}

impl crate::backend::Poller for Poller {
    fn check(&mut self) -> crate::backend::Result<Vec<NewEmail>> {
        Ok(vec![NewEmail {
            sender: "dummy@dummy.net".to_owned(),
            subject: "You Have Mail".to_owned(),
        }])
    }

    fn logout(&mut self) -> crate::backend::Result<()> {
        Ok(())
    }
}
