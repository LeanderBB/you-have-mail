use secrecy::{ExposeSecret, Secret};
use std::io::{BufRead, Read, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use you_have_mail_common::backend::Backend;
use you_have_mail_common::{
    Account, Config, DefaultEncryption, Encryption, EncryptionKey, Notification, Notifier,
    ObserverBuilder,
};

#[cfg(feature = "proton-backend")]
fn new_backed() -> Arc<dyn Backend> {
    return you_have_mail_common::backend::proton::new_backend_version_other();
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
        println!("{:?}", notification);
    }
}

fn main() {
    env_logger::init();

    let encryption_key = get_or_create_encryption_key();
    let encryptor = DefaultEncryption::new(encryption_key);
    let backend = new_backed();
    let accounts = if let Some(config) = load_config(&encryptor, &[backend.clone()]) {
        println!("Previous accounts detected");
        let mut result = Vec::with_capacity(config.accounts.len());
        for (mut a, refresher) in config.accounts {
            if let Some(r) = refresher {
                println!("Refreshing account: {}", a.email());
                a.refresh(r).unwrap();
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

        let mut account = Account::new(backend, &email, None);
        account.login(&password).unwrap();

        if account.is_awaiting_totp() {
            let mut stdout = std::io::stdout();
            let mut line_reader = std::io::BufReader::new(std::io::stdin());
            stdout.write_all("Please Input TOTP:".as_bytes()).unwrap();
            stdout.flush().unwrap();
            let mut line = String::new();
            if let Err(_) = line_reader.read_line(&mut line) {
                eprintln!("Failed to read totp");
                return;
            };

            let totp = line.trim_end_matches('\n');
            account.submit_totp(totp).unwrap();
        }

        vec![account]
    };

    println!("Starting observer...");
    let observer = ObserverBuilder::new(Box::new(StdOutNotifier {}))
        .poll_interval(Duration::from_secs(30))
        .build();

    for a in accounts {
        observer.add_account(a).unwrap();
    }

    println!("Saving observer state");
    let config = observer.generate_config().unwrap();
    write_config_file(&encryptor, config.as_bytes());

    let mut input = [0u8];
    std::io::stdin().read(&mut input).unwrap();

    println!("Saving observer state");
    let config = observer.generate_config().unwrap();
    write_config_file(&encryptor, config.as_bytes());

    println!("Goodbye");
}

fn load_config(decryptor: &DefaultEncryption, backends: &[Arc<dyn Backend>]) -> Option<Config> {
    if let Some(bytes) = load_config_file() {
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
fn load_config_file() -> Option<Vec<u8>> {
    match std::fs::read(get_config_file_path()) {
        Ok(v) => Some(v),
        Err(_) => None,
    }
}

fn write_config_file(encryptor: &DefaultEncryption, data: &[u8]) {
    let encrypted = encryptor.encrypt(data).unwrap();
    let config_path = get_config_file_dir();
    std::fs::create_dir_all(&config_path).unwrap();
    let config_file = config_path.join(CONFIG_FILE_NAME);
    std::fs::write(config_file, &encrypted).unwrap();
}
