//! You have mail implementation for proton mail accounts.

use crate::backend::{Error as BackendError, NewEmail, Result as BackendResult};
use crate::state::Account;
use crate::yhm::{IntoAccount, Yhm};
use http::{Client, Proxy};
use parking_lot::Mutex;
use proton_api::auth::{new_thread_safe_store, Auth as ProtonAuth, InMemoryStore, StoreError};
use proton_api::client::ProtonExtension;
use proton_api::domain::event::MoreEvents;
use proton_api::domain::{event, label, message, Boolean};
use proton_api::login::Sequence;
use proton_api::requests::{GetEventRequest, GetLabelsRequest, GetLatestEventRequest};
use proton_api::session::Session;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, warn, Level};

#[allow(clippy::module_name_repetitions)]
pub use proton_api;

/// Proton Mail backend.
pub struct Backend {
    base_url: Option<http::url::Url>,
    // The default client for proton servers can be shared between multiple accounts as long as
    // the process is still alive. Authentication is always read from the database, so there is no
    // risk of the clients interfering with one another.
    default_client: Mutex<Option<Arc<Client>>>,
}

impl Backend {
    /// Create a new proton mail backend.
    ///
    /// The `base_url` can be optionally overridden. If no value is specified, the default
    /// url will be used.
    #[must_use]
    pub fn new(base_url: Option<http::url::Url>) -> Arc<Self> {
        Arc::new(Backend {
            base_url,
            default_client: Mutex::new(None),
        })
    }

    /// Create a new login sequence for proton accounts.
    ///
    /// # Errors
    ///
    /// Returns error  if the http client could not be constructed.
    pub fn login_sequence(proxy: Option<Proxy>) -> http::Result<Sequence> {
        let client = new_client(proxy, None)?;
        let store = new_thread_safe_store(InMemoryStore::default());
        let session = Session::new(client, store);

        Ok(Sequence::new(session))
    }
}

pub const NAME: &str = "Proton Mail";

impl crate::backend::Backend for Backend {
    fn name(&self) -> &str {
        NAME
    }

    fn description(&self) -> &str {
        "For Proton Mail accounts (proton.me)"
    }

    fn create_client(&self, proxy: Option<Proxy>) -> BackendResult<Arc<Client>> {
        // Can't cache clients that have proxies.
        if proxy.is_some() {
            return Ok(new_client(proxy, self.base_url.as_ref())?);
        }

        // Check if the default client was built at least once.
        let mut guard = self.default_client.lock();
        if let Some(client) = &*guard {
            return Ok(Arc::clone(client));
        }

        let client = new_client(proxy, self.base_url.as_ref())?;
        *guard = Some(Arc::clone(&client));
        Ok(client)
    }

    fn new_poller(
        &self,
        client: Arc<Client>,
        account: Account,
    ) -> BackendResult<Box<dyn crate::backend::Poller>> {
        let auth = account.secret::<ProtonAuth>().map_err(|e| {
            error!("Failed to load secret state: {e}");
            e
        })?;
        let state = account.state::<TaskState>().map_err(|e| {
            error!("Failed to load state: {e}");
            e
        })?;

        let auth_store = new_thread_safe_store(AuthStore::new(account.clone(), auth));

        let session = Session::new(client, auth_store);

        let account = Poller {
            session,
            state: state.unwrap_or(TaskState::new()),
            account,
        };

        Ok(Box::new(account))
    }
}

/// Authentication store implementation for Proton.
struct AuthStore {
    account: Account,
    auth: Option<ProtonAuth>,
}

impl AuthStore {
    fn new(account: Account, auth: Option<ProtonAuth>) -> Self {
        Self { account, auth }
    }
}

impl proton_api::auth::Store for AuthStore {
    fn get(&self) -> Result<Option<&ProtonAuth>, StoreError> {
        Ok(self.auth.as_ref())
    }

    fn store(&mut self, auth: ProtonAuth) -> Result<(), StoreError> {
        self.account
            .set_secret(Some(&auth))
            .map_err(|e| StoreError::Write(anyhow::Error::new(e)))?;
        self.auth = Some(auth);
        Ok(())
    }

    fn delete(&mut self) -> Result<(), StoreError> {
        self.account
            .set_secret::<ProtonAuth>(None)
            .map_err(|e| StoreError::Write(anyhow::Error::new(e)))
    }
}

struct Poller {
    account: Account,
    session: Session,
    state: TaskState,
}

impl crate::backend::Poller for Poller {
    #[tracing::instrument(level=Level::DEBUG,skip(self),fields(email=%self.account.email()))]
    fn check(&mut self) -> BackendResult<Vec<NewEmail>> {
        let mut check_fn = || -> BackendResult<Vec<NewEmail>> {
            // First time this code is run, init state.
            if self.state.last_event_id.is_none() {
                debug!("Account is being run for the fist time, syncing resources");
                let event_id = self
                    .session
                    .execute_with_auth(GetLatestEventRequest {})
                    .map_err(|e| {
                        error!("Failed to get latest event id: {e}");
                        e
                    })?
                    .event_id;
                self.state.last_event_id = Some(event_id);

                let folders = self
                    .session
                    .execute_with_auth(GetLabelsRequest::new(label::Type::Folder))
                    .map_err(|e| {
                        error!("Failed to get custom folders: {e}");
                        e
                    })?
                    .labels;
                self.state.active_folder_ids.reserve(folders.len());
                for folder in folders {
                    if folder.notify == Boolean::True {
                        debug!("Found folder {} ({})", folder.name, folder.id);
                        self.state.active_folder_ids.insert(folder.id);
                    }
                }
                self.account.set_state(Some(&self.state)).map_err(|e| {
                    error!("Failed to store state after init: {e}");
                    e
                })?;
            }

            let mut result = EventState::new();
            if let Some(mut event_id) = self.state.last_event_id.clone() {
                let mut has_more = MoreEvents::No;
                loop {
                    let event = self
                        .session
                        .execute_with_auth(GetEventRequest::new(&event_id))
                        .map_err(|e| {
                            error!("Failed to get event: {e}");
                            e
                        })?;
                    if event.event_id != event_id || has_more == MoreEvents::Yes {
                        if let Some(label_events) = event.labels {
                            self.state.handle_label_events(label_events);
                        }

                        if let Some(message_events) = event.messages {
                            result.handle_message_events(message_events, &self.state);
                        }

                        event_id = event.event_id;
                        self.state.last_event_id = Some(event_id.clone());
                        has_more = event.more;
                    } else {
                        self.account.set_state(Some(&self.state)).map_err(|e| {
                            error!("Failed to update state after check: {e}");
                            e
                        })?;
                        return Ok(result.into());
                    }
                }
            }

            warn!("Invalid state, no event id ");
            Ok(Vec::new())
        };

        match check_fn() {
            Ok(v) => Ok(v),
            Err(e) => match e {
                BackendError::Http(http::Error::Http(code, response)) => {
                    if code == 401 {
                        return Err(BackendError::SessionExpired);
                    }
                    Err(http::Error::Http(code, response).into())
                }
                e => Err(e),
            },
        }
    }

    fn logout(&mut self) -> BackendResult<()> {
        debug!("Logging out of proton account {}", self.account.email());
        self.session.logout()?;
        Ok(())
    }
}

/// Create a new client configured for proton.
fn new_client(
    proxy: Option<Proxy>,
    base_url: Option<&http::url::Url>,
) -> http::Result<Arc<Client>> {
    let mut builder = if let Some(base_url) = base_url {
        Client::builder(base_url.clone()).allow_http()
    } else {
        Client::proton_client()
    };
    if let Some(p) = proxy {
        builder = builder.with_proxy(p);
    }

    builder
        .connect_timeout(Duration::from_secs(60))
        .request_timeout(Duration::from_secs(3 * 60))
        .build()
}

/// Contains the necessary state to process events.
#[derive(Debug, Serialize, Deserialize)]
pub struct TaskState {
    /// The last event that was processed.
    pub last_event_id: Option<event::Id>,
    /// The current list of folders that have the notification setting enabled.
    pub active_folder_ids: HashSet<label::Id>,
}

impl Default for TaskState {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskState {
    /// Create new instance.
    #[must_use]
    pub fn new() -> Self {
        Self {
            last_event_id: None,
            active_folder_ids: HashSet::from([label::Id::inbox()]),
        }
    }

    /// Create the new instance and set the last event to `id`.
    #[must_use]
    pub fn with_event_id(id: event::Id) -> Self {
        Self {
            last_event_id: Some(id),
            active_folder_ids: HashSet::from([label::Id::inbox()]),
        }
    }

    fn handle_label_events(&mut self, events: impl IntoIterator<Item = event::Label>) {
        for event in events {
            match event.action {
                event::Action::Create => {
                    if let Some(label) = event.label {
                        if label.label_type == label::Type::Folder && label.notify == Boolean::True
                        {
                            debug!("New folder: {} ({})", label.name, label.id);
                            self.active_folder_ids.insert(label.id);
                        }
                    }
                }

                event::Action::Update | event::Action::UpdateFlags => {
                    if let Some(label) = event.label {
                        if label.label_type != label::Type::Folder {
                            continue;
                        }
                        if label.notify == Boolean::True {
                            debug!("Folder {} ({}) became notifiable", label.name, label.id);
                            self.active_folder_ids.insert(label.id);
                        } else {
                            debug!("Folder {} ({}) no longer notifiable", label.name, label.id);
                            self.active_folder_ids.remove(&label.id);
                        }
                    }
                }

                event::Action::Delete => {
                    debug!("Folder {} deleted", event.id);
                    self.active_folder_ids.remove(&event.id);
                }
            }
        }
    }

    fn should_publish_notification(&self, label_list: &[label::Id]) -> bool {
        for id in label_list {
            if self.active_folder_ids.contains(id) {
                return true;
            }
        }

        false
    }
}

/// Track the state of a message in a certain event steam so that we can only display
/// a notification if no other client has opened the message.
struct EventState {
    new_emails: Vec<MessageInfo>,
    unseen: HashSet<message::Id>,
}

#[derive(Debug, Eq, PartialEq)]
struct MessageInfo {
    id: message::Id,
    sender: String,
    subject: String,
}

impl EventState {
    fn new() -> Self {
        Self {
            new_emails: Vec::new(),
            unseen: HashSet::new(),
        }
    }

    fn handle_message_events(
        &mut self,
        msg_events: impl IntoIterator<Item = event::Message>,
        state: &TaskState,
    ) {
        for msg_event in msg_events {
            match msg_event.action {
                event::Action::Create => {
                    if let Some(message) = msg_event.message {
                        // If the newly created message is not unread, it must have been read
                        // already.
                        if message.unread == Boolean::False {
                            continue;
                        }

                        // Check if the message has arrived in the inbox.
                        if state.should_publish_notification(&message.labels) {
                            self.new_emails.push(MessageInfo {
                                id: message.id.clone(),
                                subject: message.subject.clone(),
                                sender: if let Some(name) = message.sender_name {
                                    name
                                } else {
                                    message.sender_address
                                },
                            });
                            self.unseen.insert(message.id.clone());
                        }
                    }
                }
                event::Action::Update | event::Action::UpdateFlags => {
                    if let Some(message) = msg_event.message {
                        // If message switches to unread state, remove
                        if message.unread == Boolean::False {
                            self.unseen.remove(&message.id);
                        }
                    }
                }
                // Message Deleted, remove from the list.
                event::Action::Delete => {
                    self.unseen.remove(&msg_event.id);
                }
            };
        }
    }

    fn into_new_email_reply(self) -> Vec<NewEmail> {
        if self.unseen.is_empty() {
            return Vec::default();
        }

        let mut result = Vec::with_capacity(self.unseen.len());

        for msg in self.new_emails {
            if self.unseen.contains(&msg.id) {
                result.push(NewEmail {
                    sender: msg.sender,
                    subject: msg.subject,
                });
            }
        }

        result
    }
}

impl From<EventState> for Vec<NewEmail> {
    fn from(value: EventState) -> Self {
        value.into_new_email_reply()
    }
}

impl IntoAccount for Sequence {
    #[tracing::instrument(level=Level::DEBUG, skip(self, yhm))]
    fn into_account(mut self, yhm: &Yhm) -> Result<(), crate::yhm::Error> {
        let (user_info, session) = self
            .finish()
            .map_err(|_| crate::state::Error::Other(anyhow::anyhow!("Account is not logged in")))?;

        let guard = session.auth_store().read();
        let Some(auth) = guard.get().map_err(|e| {
            error!("Failed to get auth data: {e}");
            crate::state::Error::Other(anyhow::anyhow!("Failed to get authentication data: {e}"))
        })?
        else {
            error!("No authentication data available");
            return Err(crate::state::Error::Other(anyhow::anyhow!(
                "No authentication data available"
            ))
            .into());
        };

        let account = yhm.new_account(&user_info.email, NAME)?;

        account.set_secret(Some(&auth)).map_err(|e| {
            error!("Failed to set secret on account: {e}");
            e
        })?;
        account.set_proxy(session.client().proxy()).map_err(|e| {
            error!("Failed to set proxy on account: {e}");
            e
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proton_api::domain::event::Action;

    #[test]
    fn event_state_notify_unread() {
        let task_state = TaskState::new();
        let mut evt_state = EventState::new();

        let event = [event::Message {
            id: message_id(),
            action: Action::Create,
            message: Some(new_message_event_data(true, false, None)),
        }];

        evt_state.handle_message_events(event, &task_state);
        assert_eq!(evt_state.unseen.len(), 1);
        assert!(evt_state.unseen.contains(&message_id()));
        assert_eq!(evt_state.new_emails[0], message_info(false));

        let new_emails = evt_state.into_new_email_reply();
        assert_eq!(new_emails.len(), 1);
        assert_eq!(new_emails[0].sender, SENDER_ADDRESS);
        assert_eq!(new_emails[0].subject, SUBJECT);
    }

    #[test]
    fn event_state_notify_unread_multiple() {
        let task_state = TaskState::new();
        let mut evt_state = EventState::new();

        let other_id = message::Id("other".to_owned());
        let event = [
            event::Message {
                id: message_id(),
                action: Action::Create,
                message: Some(new_message_event_data(true, false, None)),
            },
            event::Message {
                id: other_id.clone(),
                action: Action::Create,
                message: Some(new_message_event_data_with_id(
                    other_id.clone(),
                    true,
                    true,
                    None,
                )),
            },
        ];

        evt_state.handle_message_events(event, &task_state);
        assert_eq!(evt_state.unseen.len(), 2);
        assert!(evt_state.unseen.contains(&message_id()));
        assert!(evt_state.unseen.contains(&other_id));
        assert_eq!(evt_state.new_emails[0], message_info(false));
        assert_eq!(
            evt_state.new_emails[1],
            message_info_with_id(other_id, true)
        );

        let new_emails = evt_state.into_new_email_reply();
        assert_eq!(new_emails.len(), 2);
        assert_eq!(new_emails[0].sender, SENDER_ADDRESS);
        assert_eq!(new_emails[0].subject, SUBJECT);
        assert_eq!(new_emails[1].sender, SENDER_NAME);
        assert_eq!(new_emails[1].subject, SUBJECT);
    }

    #[test]
    fn event_state_notify_unread_in_custom_folder() {
        let custom_folder_id = label::Id("folder".into());
        let mut task_state = TaskState::new();
        task_state
            .active_folder_ids
            .insert(custom_folder_id.clone());
        let mut evt_state = EventState::new();

        let event = [event::Message {
            id: message_id(),
            action: Action::Create,
            message: Some(new_message_event_data(true, false, Some(custom_folder_id))),
        }];

        evt_state.handle_message_events(event, &task_state);
        assert_eq!(evt_state.unseen.len(), 1);
        assert!(evt_state.unseen.contains(&message_id()));
        assert_eq!(evt_state.new_emails[0], message_info(false));

        let new_emails = evt_state.into_new_email_reply();
        assert_eq!(new_emails.len(), 1);
        assert_eq!(new_emails[0].sender, SENDER_ADDRESS);
        assert_eq!(new_emails[0].subject, SUBJECT);
    }

    #[test]
    fn event_state_notify_with_display_name_if_available() {
        let task_state = TaskState::new();
        let mut evt_state = EventState::new();

        let event = [event::Message {
            id: message_id(),
            action: Action::Create,
            message: Some(new_message_event_data(true, true, None)),
        }];

        evt_state.handle_message_events(event, &task_state);
        assert_eq!(evt_state.unseen.len(), 1);
        assert_eq!(evt_state.new_emails[0], message_info(true));

        let new_emails = evt_state.into_new_email_reply();
        assert_eq!(new_emails.len(), 1);
        assert_eq!(new_emails[0].sender, SENDER_NAME);
        assert_eq!(new_emails[0].subject, SUBJECT);
    }
    #[test]
    fn event_state_does_notify_read_message() {
        let task_state = TaskState::new();
        let mut evt_state = EventState::new();

        let event = [event::Message {
            id: message_id(),
            action: Action::Create,
            message: Some(new_message_event_data(false, false, None)),
        }];

        evt_state.handle_message_events(event, &task_state);
        assert_eq!(evt_state.unseen.len(), 0);
        assert_eq!(evt_state.new_emails.len(), 0);

        let new_emails = evt_state.into_new_email_reply();
        assert_eq!(new_emails.len(), 0);
    }

    #[test]
    fn event_state_does_notify_after_unread_message_is_read() {
        let task_state = TaskState::new();
        let mut evt_state = EventState::new();

        let event = [
            event::Message {
                id: message_id(),
                action: Action::Create,
                message: Some(new_message_event_data(false, false, None)),
            },
            event::Message {
                id: message_id(),
                action: Action::Update,
                message: Some(new_message_event_data(true, false, None)),
            },
        ];

        evt_state.handle_message_events(event, &task_state);
        assert_eq!(evt_state.unseen.len(), 0);
        assert_eq!(evt_state.new_emails.len(), 0);

        let new_emails = evt_state.into_new_email_reply();
        assert_eq!(new_emails.len(), 0);
    }

    #[test]
    fn event_state_does_notify_after_unread_message_is_deleted() {
        let task_state = TaskState::new();
        let mut evt_state = EventState::new();

        let event = [
            event::Message {
                id: message_id(),
                action: Action::Create,
                message: Some(new_message_event_data(false, false, None)),
            },
            event::Message {
                id: message_id(),
                action: Action::Delete,
                message: None,
            },
        ];

        evt_state.handle_message_events(event, &task_state);
        assert_eq!(evt_state.unseen.len(), 0);
        assert_eq!(evt_state.new_emails.len(), 0);

        let new_emails = evt_state.into_new_email_reply();
        assert_eq!(new_emails.len(), 0);
    }

    #[test]
    fn poll_state_label_event_folder_add() {
        let mut state = TaskState::new();
        // Add 3 new labels
        // Folder with notifications
        // Folder without notifications
        // Label

        let other_folder_id = label::Id("folder_without_notify".to_owned());
        let other_label_id = label::Id("label_with_notify".to_owned());
        let events = [
            event::Label {
                id: label_id(),
                action: Action::Create,
                label: Some(label_event(label_id(), label::Type::Folder, true)),
            },
            event::Label {
                id: other_folder_id.clone(),
                action: Action::Create,
                label: Some(label_event(
                    other_folder_id.clone(),
                    label::Type::Folder,
                    false,
                )),
            },
            event::Label {
                id: other_label_id.clone(),
                action: Action::Create,
                label: Some(label_event(
                    other_label_id.clone(),
                    label::Type::Label,
                    true,
                )),
            },
        ];

        state.handle_label_events(events);
        assert_eq!(state.active_folder_ids.len(), 2);
        assert!(state.active_folder_ids.contains(&label::Id::inbox()));
        assert!(state.active_folder_ids.contains(&label_id()));
        assert!(!state.active_folder_ids.contains(&other_folder_id));
        assert!(!state.active_folder_ids.contains(&other_label_id));
    }

    #[test]
    fn poll_state_label_event_folder_update() {
        let mut state = TaskState::new();

        // 3 updates
        // remove notification from default folder
        // change notification for disable folder
        // Label

        let other_folder_id = label::Id("folder_without_notify".to_owned());
        let other_label_id = label::Id("label_with_notify".to_owned());
        let events = [
            event::Label {
                id: label_id(),
                action: Action::Update,
                label: Some(label_event(label_id(), label::Type::Folder, false)),
            },
            event::Label {
                id: other_folder_id.clone(),
                action: Action::Update,
                label: Some(label_event(
                    other_folder_id.clone(),
                    label::Type::Folder,
                    true,
                )),
            },
            event::Label {
                id: other_label_id.clone(),
                action: Action::Update,
                label: Some(label_event(
                    other_label_id.clone(),
                    label::Type::Label,
                    true,
                )),
            },
        ];

        state.handle_label_events(events);
        assert_eq!(state.active_folder_ids.len(), 2);
        assert!(state.active_folder_ids.contains(&label::Id::inbox()));
        assert!(!state.active_folder_ids.contains(&label_id()));
        assert!(state.active_folder_ids.contains(&other_folder_id));
        assert!(!state.active_folder_ids.contains(&other_label_id));
    }

    #[test]
    fn poll_state_label_event_folder_delete() {
        let mut state = TaskState::new();

        let events = [
            event::Label {
                id: label_id(),
                action: Action::Create,
                label: Some(label_event(label_id(), label::Type::Folder, true)),
            },
            event::Label {
                id: label_id(),
                action: Action::Delete,
                label: None,
            },
        ];

        state.handle_label_events(events);
        assert_eq!(state.active_folder_ids.len(), 1);
        assert!(state.active_folder_ids.contains(&label::Id::inbox()));
        assert!(!state.active_folder_ids.contains(&label_id()));
    }

    fn new_message_event_data(
        unread: bool,
        with_name: bool,
        label: Option<label::Id>,
    ) -> message::Message {
        new_message_event_data_with_id(message_id(), unread, with_name, label)
    }
    fn new_message_event_data_with_id(
        id: message::Id,
        unread: bool,
        with_name: bool,
        label: Option<label::Id>,
    ) -> message::Message {
        message::Message {
            id,
            labels: vec![label.unwrap_or(label::Id::inbox())],
            subject: SUBJECT.to_owned(),
            sender_address: SENDER_ADDRESS.to_owned(),
            sender_name: if with_name {
                Some(SENDER_NAME.to_owned())
            } else {
                None
            },
            unread: if unread {
                Boolean::True
            } else {
                Boolean::False
            },
        }
    }

    fn message_info(with_name: bool) -> MessageInfo {
        message_info_with_id(message_id(), with_name)
    }

    fn message_info_with_id(id: message::Id, with_name: bool) -> MessageInfo {
        MessageInfo {
            id,
            sender: if with_name {
                SENDER_NAME.to_owned()
            } else {
                SENDER_ADDRESS.to_owned()
            },
            subject: SUBJECT.to_string(),
        }
    }

    fn message_id() -> message::Id {
        message::Id("msg".to_owned())
    }

    fn label_id() -> label::Id {
        label::Id("label".to_owned())
    }

    fn label_event(id: label::Id, label_type: label::Type, notify: bool) -> label::Label {
        label::Label {
            id,
            parent_id: None,
            name: "".to_string(),
            path: "".to_string(),
            color: "".to_string(),
            label_type,
            notify: if notify {
                Boolean::True
            } else {
                Boolean::False
            },
            display: Default::default(),
            sticky: Default::default(),
            expanded: Default::default(),
            order: 0,
        }
    }
    const SUBJECT: &str = "Hello World!";
    const SENDER_ADDRESS: &str = "bar@bar.com";
    const SENDER_NAME: &str = "Bar";
}
