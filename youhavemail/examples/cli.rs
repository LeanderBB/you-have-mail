use proton_api::domain::SecretString;
use secrecy::{ExposeSecret, SecretBox};
use sqlite_watcher::watcher::Watcher;
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tracing::info;
use you_have_mail_common::encryption::Key;
use you_have_mail_common::state::State;
use you_have_mail_common::yhm::{IntoAccount, Yhm};

fn main() {
    let filter = tracing_subscriber::EnvFilter::builder()
        .parse_lossy("info,you_have_mail_common=debug,proton_api=debug");
    tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(filter)
        .init();
    let should_quit = Arc::new(AtomicBool::new(false));
    let should_quit_copy = should_quit.clone();
    ctrlc::set_handler(move || should_quit_copy.store(true, Ordering::SeqCst))
        .expect("Failed to install ctrl+c handler");

    let encryption_key = get_or_create_encryption_key();
    let db_path = get_db_file_path();
    let watcher = Watcher::new().unwrap();
    let state = State::new(db_path, encryption_key, watcher).expect("Failed to create state");
    let yhm = Yhm::new(state);

    /*
       info!("Building observer");
       let mut observer = ObserverBuilder::new(notifier, config)
           .with_backend(backend.clone())
           .load_from_config()
           .expect("Failed to initialize observer from config");

       observer
           .set_poll_interval(Duration::from_secs(5))
           .expect("Failed to update poll interval");

    */

    if yhm.account_count().unwrap() == 0 {
        info!("No previous accounts logging in with ENV{{YHM_EMAIL}} and ENV{{YHM_PASSWORD}}");
        let email = std::env::var("YHM_EMAIL").expect("Failed to resolve env YHM_EMAIL");
        let password = SecretString::new(
            std::env::var("YHM_PASSWORD")
                .expect("Failed to resolve env YHM_PASSWORD")
                .into(),
        );

        let mut sequence =
            you_have_mail_common::backend::proton::Backend::login_sequence(None).unwrap();

        sequence
            .login(&email, password.expose_secret(), None)
            .expect("Failed to login into account");
        if sequence.is_awaiting_totp() {
            let mut stdout = std::io::stdout();
            let mut line_reader = std::io::BufReader::new(std::io::stdin());
            stdout.write_all("Please Input TOTP:".as_bytes()).unwrap();
            stdout.flush().unwrap();
            let mut line = String::new();
            line_reader
                .read_line(&mut line)
                .expect("Failed to read line");
            let totp = line.trim_end_matches('\n');
            sequence.submit_totp(totp).expect("Failed to submit totp");
        }

        sequence.into_account(&yhm).expect("Failed to add account");
    }

    info!("Starting observer loop - Ctrl+C to Quit");

    loop {
        let result = yhm.poll().expect("Failed to poll");
        if !result.is_empty() {
            println!("{result:?}")
        }
        std::thread::sleep(Duration::from_secs(5));
        if should_quit.load(Ordering::SeqCst) {
            break;
        }
    }

    info!("Goodbye");
}

fn get_or_create_encryption_key() -> SecretBox<Key> {
    let entry = keyring::Entry::new("you-have-mail-common", "secret-key-b64").unwrap();
    match entry.get_password() {
        Err(e) => {
            if !matches!(e, keyring::Error::NoEntry) {
                panic!("failed to load encryption key: {e}")
            }

            info!("No entry available, generating new key");

            let key = Key::new();
            entry
                .set_password(&key.expose_secret().to_base64())
                .unwrap();
            return key;
        }
        Ok(s) => {
            info!("Using existing key");
            Key::with_base64(s).expect("Failed to decode key")
        }
    }
}

fn get_db_file_path() -> PathBuf {
    let path = dirs::config_dir().unwrap().join("you-have-mail-common-cli");
    std::fs::create_dir_all(&path).expect("failed to create db dir");
    path.join("sqlite.db")
}
