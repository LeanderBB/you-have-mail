#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_panics_doc)]
pub mod auth;
pub mod events;
pub mod labels;
pub mod message;

pub use mockito;
use mockito::{Server, ServerOpts};

/// Create new server.
#[must_use]
pub fn new_server() -> Server {
    Server::new_with_opts(ServerOpts {
        host: "127.0.0.1",
        port: 0,
        assert_on_drop: true,
    })
}

/// Correct the mock server url for use with the client.
#[must_use]
pub fn server_url(server: &Server) -> String {
    let mut url = server.url();
    if !url.ends_with('/') {
        url.push('/');
    }
    url
}

/// Get the user id.
#[must_use]
pub fn user_id() -> &'static str {
    auth::USER_ID
}

/// Get the session UID.
#[must_use]
pub fn session_id() -> &'static str {
    auth::SESSION_UID
}

pub const DEFAULT_USER_EMAIL: &str = "foo@proton.me";
pub const DEFAULT_USER_PASSWORD: &str = "12345";
