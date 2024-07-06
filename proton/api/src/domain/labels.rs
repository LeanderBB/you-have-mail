use crate::domain::Boolean;
use serde::Deserialize;
use serde_repr::Deserialize_repr;
use std::fmt::{Display, Formatter};

/// Labels API ID. Note that label IDs are used interchangeably between what we would consider
/// mail labels and mailboxes.
#[derive(Debug, Deserialize, Eq, PartialEq, Hash, Clone)]
pub struct LabelId(pub String);

impl Display for LabelId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Deserialize_repr, Eq, PartialEq, Copy, Clone)]
#[repr(u8)]
pub enum LabelType {
    Label = 1,
    ContactGroup = 2,
    Folder = 3,
    System = 4,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Label {
    #[serde(rename = "ID")]
    pub id: LabelId,
    #[serde(rename = "ParentID")]
    pub parent_id: Option<LabelId>,
    pub name: String,
    pub path: String,
    pub color: String,
    #[serde(rename = "Type")]
    pub label_type: LabelType,
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

/// SysLabelID represents system label identifiers that are constant for every account.
#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
pub struct SysLabelId(&'static str);

impl PartialEq<LabelId> for SysLabelId {
    fn eq(&self, other: &LabelId) -> bool {
        self.0 == other.0
    }
}

impl PartialEq<SysLabelId> for LabelId {
    fn eq(&self, other: &SysLabelId) -> bool {
        self.0 == other.0
    }
}

impl From<SysLabelId> for LabelId {
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
    pub const ARCHIVE: SysLabelId = SysLabelId("5");
    pub const SENT: SysLabelId = SysLabelId("7");
    pub const DRAFTS: SysLabelId = SysLabelId("8");
    pub const OUTBOX: SysLabelId = SysLabelId("9");
    pub const STARRED: SysLabelId = SysLabelId("10");
    pub const ALL_SCHEDULED: SysLabelId = SysLabelId("12");
}

impl LabelId {
    pub fn inbox() -> Self {
        SysLabelId::INBOX.into()
    }

    pub fn all_drafts() -> Self {
        SysLabelId::ALL_DRAFTS.into()
    }

    pub fn all_sent() -> Self {
        SysLabelId::ALL_SENT.into()
    }

    pub fn trash() -> Self {
        SysLabelId::TRASH.into()
    }

    pub fn spam() -> Self {
        SysLabelId::SPAM.into()
    }

    pub fn all_mail() -> Self {
        SysLabelId::ALL_MAIL.into()
    }

    pub fn archive() -> Self {
        SysLabelId::ARCHIVE.into()
    }

    pub fn sent() -> Self {
        SysLabelId::SENT.into()
    }

    pub fn drafts() -> Self {
        SysLabelId::DRAFTS.into()
    }

    pub fn outbox() -> Self {
        SysLabelId::OUTBOX.into()
    }

    pub fn starred() -> Self {
        SysLabelId::STARRED.into()
    }

    pub fn all_scheduled() -> Self {
        SysLabelId::ALL_SCHEDULED.into()
    }
}

impl Display for SysLabelId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
