//! Glue code which combines all of the You Have Mail Components into on Service.

use crate::{
    ConfigError, Notifier, NotifierWrapper, Proxy, ServiceError, ServiceFromConfigCallback,
};
use std::ops::DerefMut;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use uniffi::deps::log::{debug, error};
use you_have_mail_common as yhm;

pub type ObserverAccountStatus = yhm::ObserverAccountStatus;

pub struct Backend(Arc<dyn yhm::backend::Backend>);

impl Backend {
    pub fn name(&self) -> String {
        self.0.name().to_string()
    }

    pub fn description(&self) -> String {
        self.0.description().to_string()
    }
}

pub type ObserverAccount = yhm::ObserverAccount;

pub struct Account {
    account: RwLock<yhm::Account>,
}

impl Account {
    pub fn email(&self) -> String {
        let accessor = self.account.read().unwrap();
        accessor.email().to_string()
    }

    pub fn is_logged_in(&self) -> bool {
        self.account.read().unwrap().is_logged_in()
    }

    pub fn is_awaiting_totp(&self) -> bool {
        self.account.read().unwrap().is_awaiting_totp()
    }

    pub fn is_logged_out(&self) -> bool {
        self.account.read().unwrap().is_logged_out()
    }

    pub fn login(&self, password: String) -> Result<(), ServiceError> {
        let mut accessor = self.account.write().unwrap();
        let account = accessor.deref_mut();
        account.login(&password)?;
        Ok(())
    }

    pub fn submit_totp(&self, totp: String) -> Result<(), ServiceError> {
        let mut accessor = self.account.write().unwrap();
        let account = accessor.deref_mut();
        account.submit_totp(&totp)?;
        Ok(())
    }

    pub fn logout(&self) -> Result<(), ServiceError> {
        let mut accessor = self.account.write().unwrap();
        let account = accessor.deref_mut();
        account.logout()?;
        Ok(())
    }
}

pub struct Service {
    observer: yhm::Observer,
    backends: Vec<Arc<Backend>>,
}

impl Service {
    pub fn new_account(
        self: Arc<Self>,
        backend: &Backend,
        email: String,
        proxy: Option<Proxy>,
    ) -> Arc<Account> {
        Arc::new(Account {
            account: RwLock::new(yhm::Account::new(backend.0.clone(), email, proxy)),
        })
    }

    pub fn get_backends(&self) -> Vec<Arc<Backend>> {
        self.backends.clone()
    }

    pub fn get_observed_accounts(&self) -> Result<Vec<ObserverAccount>, ServiceError> {
        let accounts = self.observer.get_accounts()?;
        Ok(accounts)
    }

    pub fn add_account(&self, account: Arc<Account>) -> Result<(), ServiceError> {
        let account = {
            let mut accessor = account.account.write().unwrap();
            accessor.take()
        };

        self.observer.add_account(account)?;
        Ok(())
    }

    pub fn logout_account(&self, email: String) -> Result<(), ServiceError> {
        self.observer.logout_account(email)?;
        Ok(())
    }

    pub fn remove_account(&self, email: String) -> Result<(), ServiceError> {
        self.observer.remove_account(email)?;
        Ok(())
    }

    pub fn set_account_proxy(
        &self,
        email: String,
        proxy: Option<Proxy>,
    ) -> Result<(), ServiceError> {
        self.observer.set_proxy_settings(email, proxy)?;
        Ok(())
    }

    pub fn pause(&self) -> Result<(), ServiceError> {
        self.observer.pause()?;
        Ok(())
    }

    pub fn resume(&self) -> Result<(), ServiceError> {
        self.observer.resume()?;
        Ok(())
    }

    pub fn shutdown(&self) -> Result<(), ServiceError> {
        self.observer.shutdown_worker()?;
        Ok(())
    }

    pub fn get_config(&self) -> Result<String, ConfigError> {
        let config = self.observer.generate_config()?;
        Ok(config)
    }

    pub fn get_poll_interval(&self) -> Result<u64, ServiceError> {
        let interval = self.observer.get_poll_interval()?;
        Ok(interval.as_secs())
    }

    pub fn set_poll_interval(&self, seconds: u64) -> Result<(), ServiceError> {
        let duration = Duration::from_secs(seconds);
        self.observer.set_poll_interval(duration)?;
        Ok(())
    }

    pub fn check_proxy(&self, backend: &Backend, proxy: Option<Proxy>) -> Result<(), ServiceError> {
        if let Some(p) = proxy {
            debug!(
                "Checking proxy: Protocol={} addr={} port={} auth={}",
                match p.protocol {
                    yhm::ProxyProtocol::Https => "https",
                    yhm::ProxyProtocol::Socks5 => "socks5",
                },
                p.url,
                p.port,
                p.auth.is_some(),
            );
            return backend.0.check_proxy(&p).map_err(|e| {
                error!("Failed to check proxy: {e}");
                ServiceError::ProxyError
            });
        }

        Ok(())
    }
}

pub fn new_service(notifier: Box<dyn Notifier>) -> Result<Arc<Service>, ServiceError> {
    #[cfg(target_os = "android")]
    init_android_logger();
    new_service_with_backends(notifier, get_backends(), None).map(Arc::new)
}

pub fn new_service_from_config(
    notifier: Box<dyn Notifier>,
    from_config_cb: Box<dyn ServiceFromConfigCallback>,
    bytes: &String,
) -> Result<Arc<Service>, ServiceError> {
    #[cfg(target_os = "android")]
    init_android_logger();

    let backends = get_backends();

    let config_backends = backends.iter().map(|x| x.0.clone()).collect::<Vec<_>>();

    let config =
        yhm::Config::load(&config_backends, bytes.as_bytes()).map_err(ConfigError::from)?;

    let service = new_service_with_backends(notifier, backends, Some(config.poll_interval))?;

    debug!("Found {} account(s) in config file", config.accounts.len());

    if let Err(e) = service.pause() {
        error!("Failed to pause service: {e}")
    }

    for account in config.accounts {
        debug!(
            "Refreshing account={} backend={}",
            account.0.email(),
            account.0.backend().name()
        );
        let mut account_owned = account.0;
        if let Some(refresher) = account.1 {
            if let Err(e) = account_owned.refresh(refresher) {
                error!(
                    "Refresh failed account={} backend={}: {e}",
                    account_owned.email(),
                    account_owned.backend().name()
                );
                from_config_cb.notify_error(account_owned.email().to_string(), e.into());
            }
        }

        let account_email = account_owned.email().to_string();
        if let Err(e) = service.observer.add_account(account_owned) {
            error!("Failed to add refreshed account={account_email} to observer");
            from_config_cb.notify_error(account_email, e.into());
        }
    }

    if let Err(e) = service.resume() {
        error!("Failed to resume service: {e}");
        return Err(e);
    }

    Ok(Arc::new(service))
}

fn get_backends() -> Vec<Arc<Backend>> {
    [
        #[cfg(feature = "null_backend")]
        {
            yhm::backend::null::new_backend(&[
                yhm::backend::null::NullTestAccount {
                    email: "foo".to_string(),
                    password: "foo".to_string(),
                    totp: None,
                    wait_time: Some(Duration::from_secs(2)),
                },
                yhm::backend::null::NullTestAccount {
                    email: "bar".to_string(),
                    password: "bar".to_string(),
                    totp: Some("1234".to_string()),
                    wait_time: Some(Duration::from_secs(2)),
                },
            ])
        },
        yhm::backend::proton::new_backend(),
        yhm::backend::proton::new_backend_version_other(),
    ]
    .into_iter()
    .map(|x| Arc::new(Backend(x)))
    .collect::<Vec<_>>()
}

fn new_service_with_backends(
    notifier: Box<dyn Notifier>,
    backends: Vec<Arc<Backend>>,
    poll_interval: Option<Duration>,
) -> Result<Service, ServiceError> {
    let observer = yhm::ObserverBuilder::new(Box::new(NotifierWrapper(notifier)))
        .poll_interval(poll_interval.unwrap_or(Duration::from_secs(60 * 5)))
        .build();

    Ok(Service { observer, backends })
}

#[cfg(target_os = "android")]
fn init_android_logger() {
    use android_logger::{Config, FilterBuilder};
    use uniffi::deps::log::LevelFilter;
    android_logger::init_once(
        Config::default()
            .with_max_level(LevelFilter::Debug) // limit log level
            .with_tag("yhm-rs")
            .with_filter(
                FilterBuilder::new()
                    .filter(None, LevelFilter::Error)
                    .filter(Some("you_have_mail_common"), LevelFilter::Debug)
                    .filter(Some("youhavemail::service"), LevelFilter::Debug)
                    .filter(Some("proton_api_rs"), LevelFilter::Debug)
                    .build(),
            ),
    );
}
