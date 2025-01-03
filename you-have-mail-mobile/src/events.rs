use you_have_mail_common as yhm;

/// Action that can be taken on message.
pub struct Action(String);

uniffi::custom_newtype!(Action, String);

impl From<yhm::backend::Action> for Action {
    fn from(action: yhm::backend::Action) -> Self {
        Self(action.take())
    }
}

impl From<Action> for yhm::backend::Action {
    fn from(action: Action) -> Self {
        yhm::backend::Action::with(action.0)
    }
}

/// Possible events
#[derive(uniffi::Record)]
pub struct NewEmail {
    pub sender: String,
    pub subject: String,
    pub move_to_trash_action: Option<Action>,
    pub move_to_spam_action: Option<Action>,
    pub mark_as_read_action: Option<Action>,
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
                        mark_as_read_action: v.mark_as_read_action.map(Into::into),
                        move_to_trash_action: v.move_to_trash_action.map(Into::into),
                        move_to_spam_action: v.move_to_spam_action.map(Into::into),
                    })
                    .collect(),
            },
        }
    }
}
