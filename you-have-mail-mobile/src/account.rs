use chrono::{DateTime, Local};
use you_have_mail_common as yhm;
use crate::events::Event;

/// An account in the system.
#[derive(uniffi::Object)]
pub struct Account {
    account: yhm::state::Account,
}

#[uniffi::export]
impl Account {
    pub fn email(&self) -> String {
        self.account.email().to_owned()
    }

    pub fn backend(&self) -> String {
        self.account.backend().to_string()
    }

    pub fn set_proxy(
        &self,
        proxy: Option<crate::proxy::Proxy>,
    ) -> Result<(), crate::yhm::YhmError> {
        let proxy = proxy.map(Into::into);
        Ok(self
            .account
            .set_proxy(proxy.as_ref())
            .map_err(yhm::yhm::Error::from)?)
    }

    pub fn proxy(&self) -> Result<Option<crate::proxy::Proxy>, crate::yhm::YhmError> {
        Ok(self
            .account
            .proxy()
            .map(|v| v.map(Into::into))
            .map_err(yhm::yhm::Error::from)?)
    }

    pub fn is_logged_out(&self) -> Result<bool, crate::yhm::YhmError> {
        Ok(self
            .account
            .is_logged_out()
            .map_err(yhm::yhm::Error::from)?)
    }

    pub fn last_event(&self) -> Result<Option<Event>, crate::yhm::YhmError> {
        Ok(self
            .account
            .last_event()
            .map_err(yhm::yhm::Error::from)?
            .map(Event::from))
    }

    pub fn last_poll(&self) -> Option<String> {
        self.account.last_poll().map(|dt| {
            let local = DateTime::<Local>::from(*dt);
            format!("{}", local.format("%Y/%m/%d %H:%M"))
        })
    }
}

impl Account {
    pub fn new(account: yhm::state::Account) -> Self {
        Self { account }
    }
}
