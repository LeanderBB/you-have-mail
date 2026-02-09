use crate::domain::Boolean;
use serde::{Deserialize, Serialize};
use serde_repr::Deserialize_repr;
use std::fmt::{Display, Formatter};

/// Labels API ID. Note that label IDs are used interchangeably between what we would consider
/// mail labels and mailboxes.
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Hash, Clone)]
pub struct Id(pub String);

impl Display for Id {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Type of the label.
#[derive(Debug, Deserialize_repr, Eq, PartialEq, Copy, Clone)]
#[cfg_attr(feature = "mocks", derive(serde_repr::Serialize_repr))]
#[repr(u8)]
pub enum Type {
    Label = 1,
    ContactGroup = 2,
    Folder = 3,
    System = 4,
}

/// Represents a location where you can find your messages.
#[derive(Debug, Deserialize, Eq, PartialEq, Clone)]
#[cfg_attr(feature = "mocks", derive(Serialize))]
#[serde(rename_all = "PascalCase")]
pub struct Label {
    #[serde(rename = "ID")]
    pub id: Id,
    #[serde(rename = "ParentID")]
    pub parent_id: Option<Id>,
    pub name: String,
    pub path: String,
    pub color: String,
    #[serde(rename = "Type")]
    pub label_type: Type,
    #[serde(default)]
    pub notify: Boolean,
    #[serde(default)]
    pub display: Boolean,
    #[serde(default)]
    pub sticky: Boolean,
    #[serde(default)]
    pub expanded: Boolean,
    #[serde(default = "default_label_order")]
    pub order: i32,
}

fn default_label_order() -> i32 {
    0
}

/// Represents system label identifiers that are constant for every account.
#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
pub struct SysLabelId(&'static str);

impl PartialEq<Id> for SysLabelId {
    fn eq(&self, other: &Id) -> bool {
        self.0 == other.0
    }
}

impl PartialEq<SysLabelId> for Id {
    fn eq(&self, other: &SysLabelId) -> bool {
        self.0 == other.0
    }
}

impl From<SysLabelId> for Id {
    fn from(value: SysLabelId) -> Self {
        Self(value.0.into())
    }
}

impl SysLabelId {
    pub const INBOX: SysLabelId = SysLabelId("0");
    pub const ALL_DRAFTS: SysLabelId = SysLabelId("1");
    pub const ALL_SENT: SysLabelId = SysLabelId("1");
    pub const TRASH: SysLabelId = SysLabelId("3");
    pub const SPAM: SysLabelId = SysLabelId("4");
    pub const ALL_MAIL: SysLabelId = SysLabelId("5");
    pub const ARCHIVE: SysLabelId = SysLabelId("6");
    pub const SENT: SysLabelId = SysLabelId("7");
    pub const DRAFTS: SysLabelId = SysLabelId("8");
    pub const OUTBOX: SysLabelId = SysLabelId("9");
    pub const STARRED: SysLabelId = SysLabelId("10");
    pub const ALL_SCHEDULED: SysLabelId = SysLabelId("12");
}

impl Id {
    #[must_use]
    pub fn inbox() -> Self {
        SysLabelId::INBOX.into()
    }

    #[must_use]
    pub fn all_drafts() -> Self {
        SysLabelId::ALL_DRAFTS.into()
    }

    #[must_use]
    pub fn all_sent() -> Self {
        SysLabelId::ALL_SENT.into()
    }

    #[must_use]
    pub fn trash() -> Self {
        SysLabelId::TRASH.into()
    }

    #[must_use]
    pub fn spam() -> Self {
        SysLabelId::SPAM.into()
    }

    #[must_use]
    pub fn all_mail() -> Self {
        SysLabelId::ALL_MAIL.into()
    }

    #[must_use]
    pub fn archive() -> Self {
        SysLabelId::ARCHIVE.into()
    }

    #[must_use]
    pub fn sent() -> Self {
        SysLabelId::SENT.into()
    }

    #[must_use]
    pub fn drafts() -> Self {
        SysLabelId::DRAFTS.into()
    }

    #[must_use]
    pub fn outbox() -> Self {
        SysLabelId::OUTBOX.into()
    }

    #[must_use]
    pub fn starred() -> Self {
        SysLabelId::STARRED.into()
    }

    #[must_use]
    pub fn all_scheduled() -> Self {
        SysLabelId::ALL_SCHEDULED.into()
    }
}

impl Display for SysLabelId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
