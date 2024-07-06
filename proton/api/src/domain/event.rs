use crate::domain::{Boolean, Label, LabelId};
use serde::Deserialize;
use serde_repr::Deserialize_repr;
use std::fmt::{Display, Formatter};

#[derive(Debug, Deserialize, Eq, PartialEq, Hash, Clone)]
/// Id for an API Event.
pub struct EventId(pub String);

impl Display for EventId {
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
    pub event_id: EventId,
    pub more: MoreEvents,
    pub messages: Option<Vec<MessageEvent>>,
    pub labels: Option<Vec<LabelEvent>>,
}

#[derive(Debug, Deserialize_repr, Eq, PartialEq, Copy, Clone)]
#[repr(u8)]
pub enum EventAction {
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
pub struct MessageEvent {
    #[serde(rename = "ID")]
    pub id: MessageId,
    pub action: EventAction,
    pub message: Option<Message>,
}

/// Represents an email message.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Message {
    #[serde(rename = "ID")]
    pub id: MessageId,
    #[serde(rename = "LabelIDs")]
    pub labels: Vec<LabelId>,
    pub subject: String,
    pub sender_address: String,
    pub sender_name: Option<String>,
    pub unread: Boolean,
}

/// Event data related to a Label event
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct LabelEvent {
    #[serde(rename = "ID")]
    pub id: LabelId,
    pub action: EventAction,
    pub label: Option<Label>,
}
