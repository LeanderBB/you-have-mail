use go_gpa_server::Server;
use http::Client;
use proton_api::auth::{new_thread_safe_store, InMemoryStore};
use proton_api::login::Sequence;
use proton_api::session::Session;
use std::sync::Arc;

pub const DEFAULT_USER_EMAIL: &str = "foo@bar.com";
pub const DEFAULT_USER_PASSWORD: &str = "12345";

/// Create new test server and client.
pub fn create_session_and_server() -> (Client, Server) {
    let server = Server::new().expect("failed to create test server");
    let url = server.url().expect("Failed to get server url");

    let url = url::Url::parse(&url).unwrap();
    let client = Client::builder(url)
        .allow_http()
        .build()
        .expect("Failed to build client");
    (client, server)
}

/// Perform login with `email` and `password`.
pub fn login(client: Client, email: &str, password: &str) -> Arc<Session> {
    let session = new_session(client);
    let mut sequence = Sequence::new(session);
    sequence
        .login(email, password, None)
        .expect("failed to login");
    match sequence.finish() {
        Ok(session) => session,
        Err(_) => panic!("Not logged in"),
    }
}

/// Create a new session over `client`.
pub fn new_session(client: Client) -> Arc<Session> {
    Arc::new(Session::new(
        client,
        new_thread_safe_store(InMemoryStore::default()),
    ))
}
