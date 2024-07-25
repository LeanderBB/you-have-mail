use crate::domain::event::MessageId;
use crate::domain::label::Id;
use crate::domain::Boolean;
use serde::Deserialize;

/// Represents an email message.
#[derive(Debug, Deserialize, Eq, PartialEq, Clone)]
#[cfg_attr(feature = "mocks", derive(serde::Serialize))]
#[serde(rename_all = "PascalCase")]
pub struct Message {
    #[serde(rename = "ID")]
    pub id: MessageId,
    #[serde(rename = "LabelIDs")]
    pub labels: Vec<Id>,
    pub subject: String,
    pub sender_address: String,
    pub sender_name: Option<String>,
    pub unread: Boolean,
}
