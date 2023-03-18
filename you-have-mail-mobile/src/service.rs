//! Glue code which combines all of the You Have Mail Components into on Service.

use crate::{ConfigError, Notifier, NotifierWrapper, ServiceError, ServiceFromConfigCallback};
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
    service: Arc<Service>,
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
        self.service
            .runtime
            .block_on(async { account.login(&password).await })?;
        Ok(())
    }

    pub fn submit_totp(&self, totp: String) -> Result<(), ServiceError> {
        let mut accessor = self.account.write().unwrap();
        let account = accessor.deref_mut();
        self.service
            .runtime
            .block_on(async { account.submit_totp(&totp).await })?;
        Ok(())
    }

    pub fn logout(&self) -> Result<(), ServiceError> {
        let mut accessor = self.account.write().unwrap();
        let account = accessor.deref_mut();
        self.service
            .runtime
            .block_on(async { account.logout().await })?;
        Ok(())
    }
}

pub struct Service {
    observer: yhm::Observer,
    runtime: tokio::runtime::Runtime,
    join_handle: tokio::task::JoinHandle<()>,
    backends: Vec<Arc<Backend>>,
}

impl Service {
    pub fn new_account(self: Arc<Self>, backend: &Backend, email: String) -> Arc<Account> {
        Arc::new(Account {
            account: RwLock::new(yhm::Account::new(backend.0.clone(), email)),
            service: self,
        })
    }

    pub fn get_backends(&self) -> Vec<Arc<Backend>> {
        self.backends.clone()
    }

    pub fn get_observed_accounts(&self) -> Result<Vec<ObserverAccount>, ServiceError> {
        let accounts = self
            .runtime
            .block_on(async { self.observer.get_accounts().await })?;

        Ok(accounts)
    }

    pub fn add_account(&self, account: Arc<Account>) -> Result<(), ServiceError> {
        let account = {
            let mut accessor = account.account.write().unwrap();
            accessor.take()
        };

        self.runtime
            .block_on(async { self.observer.add_account(account).await })?;
        Ok(())
    }

    pub fn logout_account(&self, email: String) -> Result<(), ServiceError> {
        self.runtime
            .block_on(async { self.observer.logout_account(email).await })?;
        Ok(())
    }

    pub fn remove_account(&self, email: String) -> Result<(), ServiceError> {
        self.runtime
            .block_on(async { self.observer.remove_account(email).await })?;
        Ok(())
    }

    pub fn pause(&self) -> Result<(), ServiceError> {
        self.runtime
            .block_on(async { self.observer.pause().await })?;
        Ok(())
    }

    pub fn resume(&self) -> Result<(), ServiceError> {
        self.runtime
            .block_on(async { self.observer.resume().await })?;
        Ok(())
    }

    pub fn shutdown(&self) -> Result<(), ServiceError> {
        self.join_handle.abort();
        Ok(())
    }

    pub fn get_config(&self) -> Result<String, ConfigError> {
        let config = self
            .runtime
            .block_on(async { self.observer.generate_config().await })?;
        Ok(config)
    }

    pub fn get_poll_interval(&self) -> Result<u64, ServiceError> {
        let interval = self
            .runtime
            .block_on(async { self.observer.get_poll_interval().await })?;
        Ok(interval.as_secs())
    }

    pub fn set_poll_interval(&self, seconds: u64) -> Result<(), ServiceError> {
        let duration = Duration::from_secs(seconds);
        self.runtime
            .block_on(async { self.observer.set_poll_interval(duration).await })?;
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

    let (poll_interval, accounts) =
        yhm::Config::load(&config_backends, bytes.as_bytes()).map_err(ConfigError::from)?;

    let service = new_service_with_backends(notifier, backends, Some(poll_interval))?;

    debug!("Found {} account(s) in config file", accounts.len());

    for account in accounts {
        debug!(
            "Refreshing account={} backend={}",
            account.0.email(),
            account.0.backend().name()
        );
        let mut account_owned = account.0;
        if let Some(refresher) = account.1 {
            if let Err(e) = service
                .runtime
                .block_on(async { account_owned.refresh(refresher).await })
            {
                error!(
                    "Refresh failed account={} backend={}: {e}",
                    account_owned.email(),
                    account_owned.backend().name()
                );
                from_config_cb.notify_error(account_owned.email().to_string(), e.into());
            }
        }

        let account_email = account_owned.email().to_string();
        if let Err(e) = service
            .runtime
            .block_on(async { service.observer.add_account(account_owned).await })
        {
            error!("Failed to add refreshed account={account_email} to observer");
            from_config_cb.notify_error(account_email, e.into());
        }
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
        yhm::backend::proton::new_backend("web-mail@5.0.17.9"),
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
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .map_err(|e| ServiceError::Unknown {
            msg: format!("Failed to start tokio runtime {e}"),
        })?;

    let (observer, task) = yhm::ObserverBuilder::new(Box::new(NotifierWrapper(notifier)))
        .poll_interval(poll_interval.unwrap_or(Duration::from_secs(30)))
        .build();
    let join_handle = runtime.spawn(task);

    Ok(Service {
        observer,
        runtime,
        join_handle,
        backends,
    })
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
                    .build(),
            ),
    );
}
