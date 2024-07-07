use proton_api::auth::{new_thread_safe_store, InMemoryStore};
use proton_api::domain::SecretString;
use proton_api::login::LoginSequence;
use proton_api::requests::{LogoutRequest, Ping, UserInfoRequest};
use proton_api::session::{Session, DEFAULT_HOST_URL};
use secrecy::ExposeSecret;
use std::io::{BufRead, Write};
use std::sync::Arc;
use tracing::metadata::LevelFilter;
use url;

fn main() {
    tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(LevelFilter::ERROR)
        .init();

    let user_email = std::env::var("USER_EMAIL").unwrap();
    let user_password = SecretString::new(std::env::var("USER_PASSWORD").unwrap());

    let base_url = url::Url::parse(DEFAULT_HOST_URL).unwrap();

    let client = http::Client::builder(base_url).debug().build().unwrap();

    let session = Arc::new(Session::new(
        client,
        new_thread_safe_store(InMemoryStore::default()),
    ));

    session.execute(Ping {}).unwrap();

    let mut login_sequence = LoginSequence::new(session.clone());

    login_sequence
        .login(&user_email, user_password.expose_secret().as_str(), None)
        .unwrap();

    if login_sequence.is_awaiting_totp() {
        let mut line_reader = std::io::BufReader::new(std::io::stdin());
        for _ in 0..3 {
            std::io::stdout()
                .write_all("Please Input TOTP:".as_bytes())
                .unwrap();
            std::io::stdout().flush().unwrap();

            let mut line = String::new();
            if let Err(e) = line_reader.read_line(&mut line) {
                eprintln!("Failed to read totp {e}");
                return;
            };

            let totp = line.trim_end_matches('\n');

            match login_sequence.submit_totp(totp) {
                Ok(()) => {
                    break;
                }
                Err(e) => {
                    eprintln!("Failed to submit totp: {e}");
                    continue;
                }
            }
        }

        if login_sequence.is_logged_in() {
            eprintln!("Failed to pass TOTP 2FA auth");
            return;
        }
    }

    let user = session.execute_with_auth(UserInfoRequest {}).unwrap();
    println!("User ID is {}", user.user.id);

    session.execute_with_auth(LogoutRequest {}).unwrap()
}
