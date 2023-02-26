use proton_api_rs::tokio;
use proton_api_rs::tokio::io::AsyncBufReadExt;
use proton_api_rs::tokio::io::AsyncWriteExt;
use std::sync::Arc;
use std::time::Duration;
use you_have_mail_common::backend::Backend;
use you_have_mail_common::{Account, AccountError, Notifier, ObserverBuilder};

#[cfg(feature = "proton-backend")]
fn new_backed() -> Arc<dyn Backend> {
    let app_version = std::env::var("YHM_PROTON_APP_VERSION").unwrap();
    return you_have_mail_common::backend::proton::new_backend(&app_version);
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
    fn notify(&self, account: &Account, email_count: usize) {
        println!(
            "Account '{}' has {email_count} new message(s)",
            account.email()
        );
    }

    fn notify_error(&self, email: &str, error: AccountError) {
        eprintln!("Account '{email}' encounter an error {error}");
    }
}

#[tokio::main(worker_threads = 1)]
async fn main() {
    let email = std::env::var("YHM_EMAIL").unwrap();
    let password = std::env::var("YHM_PASSWORD").unwrap();

    let backend = new_backed();
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

    let (observer, task) = ObserverBuilder::new(Box::new(StdOutNotifier {}))
        .poll_interval(Duration::from_secs(10))
        .build();
    let h = tokio::spawn(task);

    observer.add_account(account).await.unwrap();
    tokio::signal::ctrl_c().await.unwrap();

    h.abort();
    let _ = h.await;
}
