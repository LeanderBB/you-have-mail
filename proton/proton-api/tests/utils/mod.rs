use http::Client;
use proton_api::auth::{new_thread_safe_store, InMemoryStore};
use proton_api::domain::user::User;
use proton_api::login::Sequence;
use proton_api::mocks::auth::TFA_CODE;
use proton_api::session::Session;
use std::sync::Arc;

/// Perform login with `email` and `password`.
///
/// Set `tfa` to true to trigger Two Factor Auth.
pub fn perform_login(
    client: Arc<Client>,
    email: &str,
    password: &str,
    tfa: bool,
) -> (User, Session) {
    let session = new_session(client);
    let mut sequence = Sequence::without_server_proof_check(session);
    sequence
        .login(email, password, None)
        .expect("failed to login");
    if tfa {
        assert!(sequence.is_awaiting_totp());
        sequence
            .submit_totp(TFA_CODE)
            .expect("Failed to submit tfa");
    }

    sequence.finish().unwrap()
}

/// Create a new session over `client`.
pub fn new_session(client: Arc<Client>) -> Session {
    Session::new(client, new_thread_safe_store(InMemoryStore::default()))
}

/// Create a new client and mock server.
pub fn new_mock_session_and_server() -> (Arc<Client>, mockito::Server) {
    let server = proton_api::mocks::new_server();
    let url = url::Url::parse(&proton_api::mocks::server_url(&server)).unwrap();
    let client = Client::builder(url)
        .allow_http()
        .build()
        .expect("Failed to build client");
    (client, server)
}
