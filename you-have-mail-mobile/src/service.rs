//! Glue code which combines all of the You Have Mail Components into on Service.

use crate::{ConfigError, Notifier, NotifierWrapper, Proxy, ServiceError};
use parking_lot::{Mutex, RwLock};
use std::ops::DerefMut;
use std::path::PathBuf;
use std::sync::mpsc::{RecvTimeoutError, Sender};
use std::sync::Arc;
use std::time::Duration;
use uniffi::deps::log::{debug, error, info};
use you_have_mail_common as yhm;
use you_have_mail_common::{EncryptionKey, ExposeSecret, Secret};

#[derive(Copy, Clone)]
pub enum ObserverAccountStatus {
    Online,
    Offline,
    LoggedOut,
}

pub struct Backend(Arc<dyn yhm::backend::Backend>);

impl Backend {
    pub fn name(&self) -> String {
        self.0.name().to_string()
    }

    pub fn description(&self) -> String {
        self.0.description().to_string()
    }
}

pub struct ObserverAccount {
    pub email: String,
    pub backend: String,
    pub proxy: Option<Proxy>,
    pub status: ObserverAccountStatus,
}

pub struct Account {
    account: RwLock<yhm::Account>,
}

impl Account {
    pub fn email(&self) -> String {
        let accessor = self.account.read();
        accessor.email().to_string()
    }

    pub fn is_logged_in(&self) -> bool {
        self.account.read().is_logged_in()
    }

    pub fn is_awaiting_totp(&self) -> bool {
        self.account.read().is_awaiting_totp()
    }

    pub fn is_logged_out(&self) -> bool {
        self.account.read().is_logged_out()
    }

    pub fn login(&self, password: String, hv_data: Option<String>) -> Result<(), ServiceError> {
        let password = Secret::new(password);
        let mut accessor = self.account.write();
        let account = accessor.deref_mut();
        account.login(&password, hv_data)?;
        Ok(())
    }

    pub fn submit_totp(&self, totp: String) -> Result<(), ServiceError> {
        let mut accessor = self.account.write();
        let account = accessor.deref_mut();
        account.submit_totp(&totp)?;
        Ok(())
    }

    pub fn logout(&self) -> Result<(), ServiceError> {
        let mut accessor = self.account.write();
        let account = accessor.deref_mut();
        account.logout()?;
        Ok(())
    }
}

pub struct Service {
    observer: RwLock<yhm::Observer>,
    backends: Vec<Arc<Backend>>,
    auto_poller_sender: Mutex<Sender<AutoPollerMessage>>,
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

    pub fn get_observed_accounts(&self) -> Vec<ObserverAccount> {
        let accessor = self.observer.read();
        accessor
            .accounts()
            .map(|(email, account)| ObserverAccount {
                email: email.clone(),
                backend: account.backend().name().to_string(),
                proxy: account.get_proxy().clone(),
                status: if account.is_logged_in() {
                    ObserverAccountStatus::Online
                } else {
                    ObserverAccountStatus::LoggedOut
                },
            })
            .collect()
    }

    pub fn add_account(&self, account: Arc<Account>) -> Result<(), ServiceError> {
        let account = {
            let mut accessor = account.account.write();
            accessor.take()
        };

        self.observer.write().add_account(account)?;
        Ok(())
    }

    pub fn logout_account(&self, email: String) -> Result<(), ServiceError> {
        self.observer.write().logout_account(email)?;
        Ok(())
    }

    pub fn remove_account(&self, email: String) -> Result<(), ServiceError> {
        self.observer.write().remove_account(email)?;
        Ok(())
    }

    pub fn set_account_proxy(
        &self,
        email: String,
        proxy: Option<Proxy>,
    ) -> Result<(), ServiceError> {
        self.observer
            .write()
            .set_proxy_settings(email, proxy.as_ref())?;
        Ok(())
    }

    pub fn get_poll_interval(&self) -> u64 {
        self.observer.read().get_poll_interval().as_secs()
    }

    pub fn set_poll_interval(&self, seconds: u64) -> Result<(), ServiceError> {
        let duration = Duration::from_secs(seconds);
        self.observer.write().set_poll_interval(duration)?;
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

    pub fn pause(&self) {
        if let Err(e) = self
            .auto_poller_sender
            .lock()
            .send(AutoPollerMessage::Pause)
        {
            error!("Failed to send quit message: {e}");
        }
    }

    pub fn resume(&self) {
        if let Err(e) = self
            .auto_poller_sender
            .lock()
            .send(AutoPollerMessage::Resume)
        {
            error!("Failed to send quit message: {e}");
        }
    }

    pub fn quit(&self) {
        if let Err(e) = self.auto_poller_sender.lock().send(AutoPollerMessage::Quit) {
            error!("Failed to send quit message: {e}");
        }
    }
}

enum AutoPollerMessage {
    Pause,
    Resume,
    Quit,
}

pub fn new_service(
    notifier: Box<dyn Notifier>,
    encryption_key: String,
    config_path: String,
) -> Result<Arc<Service>, ServiceError> {
    info!("Initializing new service");
    let backends = get_backends();

    let encryption_key = Secret::new(yhm::EncryptionKey::with_base64(encryption_key).map_err(
        |e| ServiceError::Config {
            error: ConfigError::Crypto { msg: e.to_string() },
        },
    )?);
    let config = yhm::Config::create_or_load(encryption_key, config_path)
        .map_err(|e| ServiceError::Config { error: e.into() })?;

    let mut builder = yhm::ObserverBuilder::new(Arc::new(NotifierWrapper(notifier)), config);
    for b in &backends {
        builder = builder.with_backend(b.0.clone());
    }

    let observer = builder.load_from_config()?;

    let (sender, receiver) = std::sync::mpsc::channel::<AutoPollerMessage>();
    let service = Arc::new(Service {
        observer: RwLock::new(observer),
        backends,
        auto_poller_sender: Mutex::new(sender),
    });

    let s = service.clone();

    std::thread::spawn(move || {
        debug!("Starting auto-polling thread");
        let mut paused = true;
        let mut quit = false;

        fn handle_message(v: AutoPollerMessage) -> (bool, bool) {
            match v {
                AutoPollerMessage::Pause => {
                    debug!("Pausing auto poller thread");
                    (true, false)
                }
                AutoPollerMessage::Resume => {
                    debug!("Resuming auto poller thread");
                    (false, false)
                }
                AutoPollerMessage::Quit => {
                    debug!("Quiting auto poller thread");
                    (true, true)
                }
            }
        }

        while !quit {
            if paused {
                match receiver.recv() {
                    Ok(v) => {
                        (paused, quit) = handle_message(v);
                    }
                    Err(_) => return,
                }
            } else {
                {
                    if let Err(e) = s.observer.read().poll() {
                        error!("Failed to poll: {e}");
                    }
                }

                let interval = { s.observer.read().get_poll_interval() };

                match receiver.recv_timeout(interval) {
                    Ok(v) => {
                        (paused, quit) = handle_message(v);
                    }
                    Err(e) => match e {
                        RecvTimeoutError::Timeout => {
                            continue;
                        }
                        RecvTimeoutError::Disconnected => {
                            return;
                        }
                    },
                }
            }
        }
        debug!("Auto poller thread finished");
    });

    Ok(service)
}

pub fn new_encryption_key() -> String {
    EncryptionKey::new().expose_secret().to_base64()
}

pub fn migrate_old_config(
    encryption_key: String,
    config: String,
    file_path: String,
) -> Result<(), ServiceError> {
    let key = Secret::new(EncryptionKey::with_base64(encryption_key).map_err(|e| {
        ServiceError::Config {
            error: ConfigError::Crypto {
                msg: format!("Invalid crypto key:{e}"),
            },
        }
    })?);
    let encrypted = yhm::encrypt(&key, config.as_bytes()).map_err(|e| ServiceError::Config {
        error: ConfigError::Crypto { msg: e.to_string() },
    })?;
    std::fs::write(file_path, encrypted.as_slice()).map_err(|e| ServiceError::Config {
        error: ConfigError::IO { msg: e.to_string() },
    })?;
    Ok(())
}

pub fn init_log(filepath: String) -> Option<String> {
    if let Err(e) = you_have_mail_common::log::init_log(PathBuf::from(filepath)) {
        return Some(e.to_string());
    }
    info!("Log file initialized");
    None
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
                    refresh: false,
                },
                yhm::backend::null::NullTestAccount {
                    email: "bar".to_string(),
                    password: "bar".to_string(),
                    totp: Some("1234".to_string()),
                    wait_time: Some(Duration::from_secs(2)),
                    refresh: false,
                },
            ])
        },
        yhm::backend::proton::new_backend(),
    ]
    .into_iter()
    .map(|x| Arc::new(Backend(x)))
    .collect::<Vec<_>>()
}
