//! You have mail implementation for proton mail accounts.

use crate::backend::{Backend, Error as BackendError, NewEmail, Poller, Result as BackendResult};
use crate::encryption::Key;
use crate::state::{Account, IntoAccount, State};
use http::{Client, Proxy};
use proton_api::auth::{new_thread_safe_store, Auth as ProtonAuth, InMemoryStore, StoreError};
use proton_api::client::ProtonExtension;
use proton_api::domain::event::{MessageId, MoreEvents};
use proton_api::domain::label::Type;
use proton_api::domain::{event, label, Boolean};
use proton_api::login::Sequence;
use proton_api::requests::{GetEventRequest, GetLabelsRequest, GetLatestEventRequest};
use proton_api::session::Session;
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, warn, Level};

/// Create a proton mail backend.
pub fn new_backend(db: Arc<State>) -> Arc<dyn Backend> {
    Arc::new(ProtonBackend { db })
}

pub fn new_login_sequence(proxy: Option<Proxy>) -> http::Result<Sequence> {
    let client = new_client(proxy)?;
    let store = new_thread_safe_store(InMemoryStore::default());
    let session = Session::new(client, store);

    Ok(Sequence::new(session))
}

struct ProtonBackend {
    db: Arc<State>,
}

pub const PROTON_BACKEND_NAME: &str = "Proton Mail";
pub const PROTON_BACKEND_NAME_OTHER: &str = "Proton Mail V-Other";

impl Backend for ProtonBackend {
    fn name(&self) -> &str {
        PROTON_BACKEND_NAME
    }

    fn description(&self) -> &str {
        "For Proton accounts (mail.proton.com)"
    }

    fn create_client(&self, proxy: Option<Proxy>) -> BackendResult<Arc<Client>> {
        Ok(new_client(proxy)?)
    }

    fn new_poller(&self, client: Arc<Client>, account: &Account) -> BackendResult<Box<dyn Poller>> {
        let auth = account
            .secret::<ProtonAuth>(self.db.encryption_key().expose_secret())
            .map_err(|e| {
                error!("Failed to load secret state: {e}");
                e
            })?;
        let state = account.state::<TaskState>().map_err(|e| {
            error!("Failed to load state: {e}");
            e
        })?;

        let auth_store = new_thread_safe_store(AuthStore::new(
            account.email().to_owned(),
            auth,
            Arc::clone(&self.db),
        ));

        let session = Session::new(client, auth_store);

        let account = ProtonPoller {
            email: account.email().to_owned(),
            session,
            state: state.unwrap_or(TaskState::new()),
            db: Arc::clone(&self.db),
        };

        Ok(Box::new(account))
    }
}

/// Authentication store implementation for Proton.
struct AuthStore {
    email: String,
    auth: Option<ProtonAuth>,
    db: Arc<State>,
}

impl AuthStore {
    fn new(email: String, auth: Option<ProtonAuth>, db: Arc<State>) -> Self {
        Self { email, auth, db }
    }
}

impl proton_api::auth::Store for AuthStore {
    fn get(&self) -> Result<Option<&ProtonAuth>, StoreError> {
        Ok(self.auth.as_ref())
    }

    fn store(&mut self, auth: ProtonAuth) -> Result<(), StoreError> {
        self.db
            .update_secret_state(&self.email, &auth)
            .map_err(|e| StoreError::Write(anyhow::Error::new(e)))?;
        self.auth = Some(auth);
        Ok(())
    }

    fn delete(&mut self) -> Result<(), StoreError> {
        self.db
            .delete_secret_state(&self.email)
            .map_err(|e| StoreError::Write(anyhow::Error::new(e)))
    }
}

struct ProtonPoller {
    email: String,
    session: Session,
    state: TaskState,
    db: Arc<State>,
}

impl Poller for ProtonPoller {
    #[tracing::instrument(level=Level::DEBUG,skip(self),fields(email=%self.email))]
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
                    .execute_with_auth(GetLabelsRequest::new(Type::Folder))
                    .map_err(|e| {
                        error!("Failed to get custom folders: {e}");
                        e
                    })?
                    .labels;
                self.state.active_folder_ids.reserve(folders.len());
                for folder in folders {
                    if folder.notify == Boolean::True {
                        self.state.active_folder_ids.insert(folder.id);
                    }
                }
                self.db
                    .update_account_state(&self.email, &self.state)
                    .map_err(|e| {
                        error!("Failed to store state after init: {e}");
                        e
                    })?;
                debug!(
                    "Account has following list of custom folders: {:?}",
                    self.state.active_folder_ids
                )
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
                        if let Some(label_events) = &event.labels {
                            self.state.handle_label_events(label_events)
                        }

                        if let Some(message_events) = &event.messages {
                            result.handle_message_events(message_events, &self.state);
                        }

                        event_id = event.event_id;
                        self.state.last_event_id = Some(event_id.clone());
                        has_more = event.more;
                    } else {
                        return Ok(result.into());
                    }
                }
            }

            warn!("Invalid state, no event id ");
            Ok(Vec::new())
        };

        match check_fn() {
            Ok(v) => {
                self.db
                    .update_account_state(&self.email, &self.state)
                    .map_err(|e| {
                        error!("Failed to update state after check: {e}");
                        e
                    })?;
                Ok(v)
            }
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
        debug!("Logging out of proton account {}", self.email);
        self.session.logout()?;
        Ok(())
    }
}

/// Create a new client configured for proton.
fn new_client(proxy: Option<Proxy>) -> http::Result<Arc<Client>> {
    let mut builder = Client::proton_client();
    if let Some(p) = proxy {
        builder = builder.with_proxy(p)
    }

    builder
        .connect_timeout(Duration::from_secs(60))
        .request_timeout(Duration::from_secs(3 * 60))
        .build()
}

struct MsgInfo {
    id: MessageId,
    sender: String,
    subject: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct TaskState {
    last_event_id: Option<event::Id>,
    active_folder_ids: HashSet<label::Id>,
}

impl TaskState {
    fn new() -> Self {
        Self {
            last_event_id: None,
            active_folder_ids: HashSet::from([label::Id::inbox()]),
        }
    }

    fn handle_label_events(&mut self, events: &[event::Label]) {
        for event in events {
            match event.action {
                event::Action::Create => {
                    if let Some(label) = event.label.as_ref() {
                        if label.notify == Boolean::True {
                            debug!("New custom folder added to notification list: {}", label.id);
                            self.active_folder_ids.insert(label.id.clone());
                        }
                    }
                }

                event::Action::Update | event::Action::UpdateFlags => {
                    if let Some(label) = event.label.as_ref() {
                        if label.notify == Boolean::True {
                            debug!("Folder {} added to notification list", label.id);
                            self.active_folder_ids.insert(label.id.clone());
                        } else {
                            debug!("Folder {} removed from notification list", label.id);
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
    new_emails: Vec<MsgInfo>,
    unseen: HashSet<MessageId>,
}

impl EventState {
    fn new() -> Self {
        Self {
            new_emails: Vec::new(),
            unseen: HashSet::new(),
        }
    }

    fn handle_message_events(&mut self, msg_events: &[event::Message], state: &TaskState) {
        for msg_event in msg_events {
            match msg_event.action {
                event::Action::Create => {
                    if let Some(message) = &msg_event.message {
                        // If the newly created message is not unread, it must have been read
                        // already.
                        if message.unread == Boolean::False {
                            return;
                        }

                        // Check if the message has arrived in the inbox.
                        if state.should_publish_notification(&message.labels) {
                            self.new_emails.push(MsgInfo {
                                id: message.id.clone(),
                                subject: message.subject.clone(),
                                sender: if let Some(name) = &message.sender_name {
                                    name.clone()
                                } else {
                                    message.sender_address.clone()
                                },
                            });
                            self.unseen.insert(message.id.clone());
                        }
                    }
                }
                event::Action::Update | event::Action::UpdateFlags => {
                    if let Some(message) = &msg_event.message {
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
                })
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
    fn into_account(self, encryption_key: &Key) -> Result<Account, crate::state::Error> {
        let (user_info, session) = self
            .finish()
            .map_err(|_| crate::state::Error::Other(anyhow::anyhow!("Account is not logged in")))?;

        let guard = session.auth_store().read();
        let Some(auth) = guard.get().map_err(|e| {
            crate::state::Error::Other(anyhow::anyhow!("Failed to get authentication data: {e}"))
        })?
        else {
            return Err(crate::state::Error::Other(anyhow::anyhow!(
                "No authentication data available"
            )));
        };

        let mut account = Account::new(user_info.email, PROTON_BACKEND_NAME.to_owned());

        account.set_secret(encryption_key, Some(&auth))?;
        account.set_proxy(encryption_key, session.client().proxy())?;

        Ok(account)
    }
}
