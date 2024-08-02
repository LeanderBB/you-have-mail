use crate::backend::{Error, NewEmail};
use crate::yhm::PollOutput;
use rusqlite::types::{FromSql, FromSqlResult, ToSqlOutput, Value, ValueRef};
use rusqlite::ToSql;
use serde::{Deserialize, Serialize};
use tracing::error;

/// Possible events
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
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
    Error(String, String),
}
impl Event {
    /// Get the email of the account associated with th event
    #[must_use]
    pub fn email(&self) -> &str {
        match self {
            Event::NewEmail { email, .. }
            | Event::LoggedOut(email)
            | Event::Offline(email)
            | Event::Error(email, _) => email.as_str(),
        }
    }
}

impl Event {
    pub(crate) fn new(value: &PollOutput) -> Self {
        match &value.result {
            Ok(new_email) => Self::NewEmail {
                email: value.email.clone(),
                backend: value.backend.clone(),
                emails: new_email.clone(),
            },
            Err(e) => match e {
                Error::Http(http_err) => {
                    if http_err.is_connection_error() {
                        return Self::Offline(value.email.clone());
                    }
                    Self::Error(value.email.clone(), e.to_string())
                }
                Error::SessionExpired => Self::LoggedOut(value.email.clone()),
                err => Self::Error(value.email.clone(), err.to_string()),
            },
        }
    }
}

impl ToSql for Event {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        let value = serde_json::to_string(self)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.into()))?;

        Ok(ToSqlOutput::Owned(Value::Text(value)))
    }
}

impl FromSql for Event {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let value = value.as_str()?;
        let event = serde_json::from_str(value).map_err(|e| {
            error!("Failed to deserialize event: {e}");
            rusqlite::types::FromSqlError::InvalidType
        })?;
        Ok(event)
    }
}
