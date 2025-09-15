use proton_api::auth::{InMemoryStore, new_thread_safe_store};
use proton_api::client::ProtonExtension;
use proton_api::domain::SecretString;
use proton_api::login::Sequence;
use proton_api::requests::{GetUserInfoRequest, Ping};
use proton_api::session::Session;
use secrecy::ExposeSecret;
use std::io::{BufRead, Write};
use tracing::metadata::LevelFilter;

fn main() {
    tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(LevelFilter::ERROR)
        .init();

    let user_email = std::env::var("USER_EMAIL").unwrap();
    let user_password = SecretString::new(std::env::var("USER_PASSWORD").unwrap().into());

    let client = you_have_mail_http::Client::proton_client()
        .debug()
        .build()
        .unwrap();

    let session = Session::new(client, new_thread_safe_store(InMemoryStore::default()));

    session.execute(Ping {}).unwrap();

    let mut login_sequence = Sequence::new(session.clone());

    login_sequence
        .login(&user_email, user_password.expose_secret(), None)
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

    let user = session.execute_with_auth(GetUserInfoRequest {}).unwrap();
    println!("User ID is {}", user.user.id);

    session.logout().unwrap();
}
