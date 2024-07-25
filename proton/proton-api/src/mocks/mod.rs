pub mod auth;
pub mod events;
pub mod labels;

pub use mockito;
use mockito::{Server, ServerOpts};
/// Create new server.
pub fn new() -> Server {
    Server::new_with_opts(ServerOpts {
        host: "127.0.0.1",
        port: 0,
        assert_on_drop: true,
    })
}

/// Get the user id.
pub fn user_id() -> &'static str {
    auth::USER_ID
}

/// Get the session UID.
pub fn session_id() -> &'static str {
    auth::USER_ID
}
