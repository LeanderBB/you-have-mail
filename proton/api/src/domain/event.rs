use serde::Deserialize;
use serde_repr::Deserialize_repr;
use std::fmt::{Display, Formatter};

#[derive(Debug, Deserialize, Eq, PartialEq, Hash, Clone)]
/// Id for an API Event.
pub struct Id(pub String);

impl Display for Id {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Deserialize_repr, Eq, PartialEq, Copy, Clone)]
#[repr(u8)]
pub enum MoreEvents {
    No = 0,
    Yes = 1,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Event {
    #[serde(rename = "EventID")]
    pub event_id: Id,
    pub more: MoreEvents,
    pub messages: Option<Vec<Message>>,
    pub labels: Option<Vec<Label>>,
}

#[derive(Debug, Deserialize_repr, Eq, PartialEq, Copy, Clone)]
#[repr(u8)]
pub enum Action {
    Delete = 0,
    Create = 1,
    Update = 2,
    UpdateFlags = 3,
}

/// Message API ID.
#[derive(Debug, Deserialize, Eq, PartialEq, Hash, Clone)]
pub struct MessageId(String);

impl Display for MessageId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Event data related to a Message event.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Message {
    #[serde(rename = "ID")]
    pub id: MessageId,
    pub action: Action,
    pub message: Option<crate::domain::message::Message>,
}

/// Event data related to a Label event
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Label {
    #[serde(rename = "ID")]
    pub id: Id,
    pub action: Action,
    pub label: Option<crate::domain::label::Label>,
}
