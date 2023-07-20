use proton_api_rs::domain::SecretString;
use proton_api_rs::log::{error, info, warn};
use secrecy::{ExposeSecret, Secret};
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use you_have_mail_common::backend::Backend;
use you_have_mail_common::{
    Account, Config, ConfigResult, EncryptionKey, Notification, Notifier, ObserverBuilder,
};

#[cfg(feature = "proton-backend")]
fn new_backed() -> Arc<dyn Backend> {
    return you_have_mail_common::backend::proton::new_backend();
}

#[cfg(not(feature = "proton-backend"))]
fn new_backed() -> Arc<dyn Backend> {
    use you_have_mail_common::backend::null::NullTestAccount;
    return you_have_mail_common::backend::null::new_backend(&[NullTestAccount {
        email: "foo".to_string(),
        password: "bar".to_string(),
        totp: None,
        proxy: None,
    }]);
}

struct StdOutNotifier {}

impl Notifier for StdOutNotifier {
    fn notify<'a>(&self, notification: Notification) {
        match notification {
            Notification::NewEmail {
                account,
                backend,
                emails,
            } => {
                info!("{} new email(s) for {account} on {backend}", emails.len());
                for info in emails {
                    info!("--> Sender={} Subject={}", info.sender, info.subject);
                }
            }
            Notification::AccountAdded(email, backend, _) => {
                info!("Account Added {email} ({backend})");
            }
            Notification::AccountLoggedOut(email) => {
                info!("Account Logged out {email}");
            }
            Notification::AccountRemoved(email) => {
                info!("Account Removed {email}");
            }
            Notification::AccountOffline(email) => {
                warn!("Account Offline {email}");
            }
            Notification::AccountOnline(email) => {
                info!("Account Online {email}");
            }
            Notification::AccountError(email, e) => {
                error!("Account {email}: {e}");
            }
            Notification::ProxyApplied(email, _) => {
                info!("Account {email} proxy changed");
            }
            Notification::ConfigError(e) => {
                error!("Config Error: {e}");
            }
            Notification::Error(e) => {
                error!("{e}");
            }
        }
    }
}

fn main() {
    env_logger::init();
    let should_quit = Arc::new(AtomicBool::new(false));
    let should_quit_copy = should_quit.clone();
    ctrlc::set_handler(move || should_quit_copy.store(true, Ordering::SeqCst))
        .expect("Failed to install ctrl+c handler");

    let encryption_key = get_or_create_encryption_key();
    let backend = new_backed();
    let notifier = Arc::new(StdOutNotifier {});

    info!("Loading config");
    let config = load_config(encryption_key).expect("Failed to load config");

    info!("Building observer");
    let mut observer = ObserverBuilder::new(notifier, config)
        .with_backend(backend.clone())
        .load_from_config()
        .expect("Failed to initialize observer from config");

    observer
        .set_poll_interval(Duration::from_secs(5))
        .expect("Failed to update poll interval");

    if observer.len() == 0 {
        info!("No previous accounts logging in with ENV{{YHM_EMAIL}} and ENV{{YHM_PASSWORD}}");
        let email = std::env::var("YHM_EMAIL").expect("Failed to resolve env YHM_EMAIL");
        let password = SecretString::new(
            std::env::var("YHM_PASSWORD").expect("Failed to resolve env YHM_PASSWORD"),
        );

        let mut account = Account::new(backend, &email, None);
        account
            .login(&password, None)
            .expect("Failed to login into account");

        if account.is_awaiting_totp() {
            let mut stdout = std::io::stdout();
            let mut line_reader = std::io::BufReader::new(std::io::stdin());
            stdout.write_all("Please Input TOTP:".as_bytes()).unwrap();
            stdout.flush().unwrap();
            let mut line = String::new();
            line_reader
                .read_line(&mut line)
                .expect("Failed to read line");
            let totp = line.trim_end_matches('\n');
            account.submit_totp(totp).expect("Failed to submit totp");
        }

        observer
            .add_account(account)
            .expect("Failed to add account");
    }

    info!("Starting observer loop - Ctrl+C to Quit");

    loop {
        observer.poll().expect("Failed to poll");
        std::thread::sleep(observer.get_poll_interval());
        if should_quit.load(Ordering::SeqCst) {
            break;
        }
    }

    info!("Goodbye");
}

fn load_config(encryption_key: Secret<EncryptionKey>) -> ConfigResult<Config> {
    let config_path = get_config_file_path();
    Config::create_or_load(encryption_key, config_path)
}

fn get_or_create_encryption_key() -> Secret<EncryptionKey> {
    let entry = keyring::Entry::new("you-have-mail-common", "secret-key-b64").unwrap();
    match entry.get_password() {
        Err(e) => {
            if !matches!(e, keyring::Error::NoEntry) {
                panic!("failed to load encryption key: {e}")
            }

            let key = EncryptionKey::new();
            entry
                .set_password(&key.expose_secret().to_base64())
                .unwrap();
            return key;
        }
        Ok(s) => {
            let key = EncryptionKey::with_base64(s).expect("Failed to decode key");
            Secret::new(key)
        }
    }
}

const CONFIG_FILE_NAME: &str = "you-have-mail-common-cli.conf";

fn get_config_file_path() -> PathBuf {
    dirs::config_dir().unwrap().join(CONFIG_FILE_NAME)
}
