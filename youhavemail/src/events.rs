use crate::backend::{Error, NewEmail};
use crate::yhm::PollOutput;

/// Possible events
pub enum Event {
    /// One or more emails has arrived.
    NewEmail {
        email: String,
        backend: String,
        emails: Vec<NewEmail>,
    },
    /// Account has been logged out.
    LoggedOut(String),
    /// Account servers are not reachable.
    Offline(String),
    /// General error occurred.
    Error(String, Error),
}

impl From<PollOutput> for Event {
    fn from(value: PollOutput) -> Self {
        match value.result {
            Ok(new_email) => Self::NewEmail {
                email: value.email,
                backend: value.backend,
                emails: new_email,
            },
            Err(e) => match e {
                Error::Http(e) => {
                    if e.is_connection_error() {
                        return Self::Offline(value.email);
                    }
                    Self::Error(value.email, Error::Http(e))
                }
                Error::SessionExpired => Self::LoggedOut(value.email),
                err => Self::Error(value.email, err),
            },
        }
    }
}
