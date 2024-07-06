use proton_api::clientv2::{ping, SessionType};
use proton_api::domain::SecretString;
use proton_api::http::Sequence;
use proton_api::{http, Session};
use std::io::{BufRead, Write};

fn main() {
    env_logger::init();

    let user_email = std::env::var("PAPI_USER_EMAIL").unwrap();
    let user_password = SecretString::new(std::env::var("PAPI_USER_PASSWORD").unwrap());
    let app_version = std::env::var("PAPI_APP_VERSION").unwrap();

    let client = http::ClientBuilder::new()
        .app_version(&app_version)
        .debug()
        .build::<http::ureq_client::UReqClient>()
        .unwrap();

    ping().do_sync(&client).unwrap();

    let login_result = Session::login(&user_email, &user_password, None).do_sync(&client);
    let session = match login_result.unwrap() {
        SessionType::Authenticated(s) => s,
        SessionType::AwaitingTotp(t) => {
            let mut line_reader = std::io::BufReader::new(std::io::stdin());
            let session = {
                let mut session = None;
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

                    match t.submit_totp(totp).do_sync(&client) {
                        Ok(ac) => {
                            session = Some(ac);
                            break;
                        }
                        Err(e) => {
                            eprintln!("Failed to submit totp: {e}");
                            continue;
                        }
                    }
                }

                session
            };

            let Some(c) = session else {
                eprintln!("Failed to pass TOTP 2FA auth");
                return;
            };
            c
        }
    };

    let user = session.get_user().do_sync(&client).unwrap();
    println!("User ID is {}", user.id);

    session.logout().do_sync(&client).unwrap();
}
