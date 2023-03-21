use proton_api_rs::tokio;
use proton_api_rs::tokio::io::AsyncBufReadExt;
use proton_api_rs::tokio::io::AsyncWriteExt;
use secrecy::{ExposeSecret, Secret};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use you_have_mail_common::backend::Backend;
use you_have_mail_common::{
    Account, Config, ConfigAccount, DefaultEncryption, Encryption, EncryptionKey, Notification,
    Notifier, ObserverBuilder,
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
    }]);
}

struct StdOutNotifier {}

impl Notifier for StdOutNotifier {
    fn notify<'a>(&self, notification: Notification) {
        println!("{:?}", notification);
    }
}

#[tokio::main(worker_threads = 1)]
async fn main() {
    let encryption_key = get_or_create_encryption_key();
    let encryptor = DefaultEncryption::new(encryption_key);
    let backend = new_backed();
    let accounts = if let Some((_, accounts)) = load_config(&encryptor, &[backend.clone()]).await {
        println!("Previous accounts detected");
        let mut result = Vec::with_capacity(accounts.len());
        for (mut a, refresher) in accounts {
            if let Some(r) = refresher {
                println!("Refreshing account: {}", a.email());
                a.refresh(r).await.unwrap();
            } else {
                println!("Account {} is logged out", a.email());
            }
            result.push(a);
        }
        result
    } else {
        println!("No previous accounts logging in with ENV{{YHM_EMAIL}} and ENV{{YHM_PASSWORD}}");
        let email = std::env::var("YHM_EMAIL").unwrap();
        let password = std::env::var("YHM_PASSWORD").unwrap();

        let mut account = Account::new(backend, &email);
        account.login(&password).await.unwrap();

        if account.is_awaiting_totp() {
            let mut stdout = tokio::io::stdout();
            let mut line_reader = tokio::io::BufReader::new(tokio::io::stdin()).lines();
            stdout
                .write_all("Please Input TOTP:".as_bytes())
                .await
                .unwrap();
            stdout.flush().await.unwrap();

            let Some(line) = line_reader.next_line().await.unwrap() else {
                eprintln!("Failed to read totp");
                return;
            };

            let totp = line.trim_end_matches('\n');
            account.submit_totp(totp).await.unwrap();
        }

        vec![account]
    };

    let (observer, task) = ObserverBuilder::new(Box::new(StdOutNotifier {}))
        .poll_interval(Duration::from_secs(10))
        .build();

    println!("Starting observer...");
    let h = tokio::spawn(task);

    for a in accounts {
        observer.add_account(a).await.unwrap();
    }

    tokio::signal::ctrl_c().await.unwrap();

    println!("Saving observer state");
    let config = observer.generate_config().await.unwrap();
    write_config_file(&encryptor, config.as_bytes()).await;

    h.abort();
    let _ = h.await;
    println!("Goodbye");
}

async fn load_config(
    decryptor: &DefaultEncryption,
    backends: &[Arc<dyn Backend>],
) -> Option<(Duration, Vec<ConfigAccount>)> {
    if let Some(bytes) = load_config_file().await {
        let decrypted = decryptor.decrypt(&bytes).unwrap();

        return Some(Config::load(&backends, &decrypted).unwrap());
    }
    None
}

fn get_or_create_encryption_key() -> Secret<EncryptionKey> {
    let entry = keyring::Entry::new("you-have-mail-common", "secret-key").unwrap();
    match entry.get_password() {
        Err(e) => {
            if !matches!(e, keyring::Error::NoEntry) {
                panic!("failed to load encryption key: {e}")
            }

            let key = EncryptionKey::new();
            entry
                .set_password(&hex::encode(key.expose_secret()))
                .unwrap();
            return key;
        }
        Ok(s) => {
            let bytes = hex::decode(s).unwrap();
            let fixed: [u8; 32] = bytes.try_into().unwrap();
            Secret::new(EncryptionKey::from(fixed))
        }
    }
}

fn get_config_file_dir() -> PathBuf {
    dirs::config_dir().unwrap()
}

const CONFIG_FILE_NAME: &str = "you-have-mail-common-cli.conf";

fn get_config_file_path() -> PathBuf {
    dirs::config_dir().unwrap().join(CONFIG_FILE_NAME)
}
async fn load_config_file() -> Option<Vec<u8>> {
    match tokio::fs::read(get_config_file_path()).await {
        Ok(v) => Some(v),
        Err(_) => None,
    }
}

async fn write_config_file(encryptor: &DefaultEncryption, data: &[u8]) {
    let encrypted = encryptor.encrypt(data).unwrap();
    let config_path = get_config_file_dir();
    tokio::fs::create_dir_all(&config_path).await.unwrap();
    let config_file = config_path.join(CONFIG_FILE_NAME);
    tokio::fs::write(config_file, &encrypted).await.unwrap();
}
