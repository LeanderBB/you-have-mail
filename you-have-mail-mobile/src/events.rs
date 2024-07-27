use you_have_mail_common as yhm;

/// Possible events

#[derive(uniffi::Record)]
pub struct NewEmail {
    pub sender: String,
    pub subject: String,
}
#[derive(uniffi::Enum)]
pub enum Event {
    /// One or more emails has arrived.
    Email {
        email: String,
        backend: String,
        emails: Vec<NewEmail>,
    },
    /// Account has been logged out.
    LoggedOut(String),
    /// Account servers are not reachable.
    Offline(String),
    /// General error occurred.
    Error(String, String),
}

impl From<yhm::events::Event> for Event {
    fn from(value: yhm::events::Event) -> Self {
        match value {
            yhm::events::Event::Error(email, e) => Self::Error(email, e.to_string()),
            yhm::events::Event::Offline(email) => Self::Offline(email),
            yhm::events::Event::LoggedOut(email) => Self::LoggedOut(email),
            yhm::events::Event::NewEmail {
                email,
                backend,
                emails,
            } => Self::Email {
                email,
                backend,
                emails: emails
                    .into_iter()
                    .map(|v| NewEmail {
                        sender: v.sender,
                        subject: v.subject,
                    })
                    .collect(),
            },
        }
    }
}
