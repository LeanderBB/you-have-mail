use crate::domain::label::Id as LabelId;
use crate::domain::Boolean;
use serde::Deserialize;
use std::fmt::{Display, Formatter};

/// Message API ID.
#[derive(Debug, Deserialize, Eq, PartialEq, Hash, Clone)]
#[cfg_attr(feature = "mocks", derive(serde::Serialize))]
pub struct Id(pub String);

impl Display for Id {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Represents an email message.
#[derive(Debug, Deserialize, Eq, PartialEq, Clone)]
#[cfg_attr(feature = "mocks", derive(serde::Serialize))]
#[serde(rename_all = "PascalCase")]
pub struct Message {
    #[serde(rename = "ID")]
    pub id: Id,
    #[serde(rename = "LabelIDs")]
    pub labels: Vec<LabelId>,
    pub subject: String,
    pub sender_address: String,
    pub sender_name: Option<String>,
    pub unread: Boolean,
}
