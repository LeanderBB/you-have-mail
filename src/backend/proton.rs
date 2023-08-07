//! You have mail implementation for proton mail accounts.

use crate::backend::{
    Account, AccountRefreshedNotifier, AuthRefresher, AwaitTotp, Backend, BackendError,
    BackendResult, CheckTask, EmailInfo, NewEmailReply,
};
use crate::{AccountState, Proxy, ProxyProtocol};
use anyhow::{anyhow, Error};
use parking_lot::Mutex;
use proton_api_rs::domain::{
    Boolean, EventAction, EventId, ExposeSecret, HumanVerificationLoginData, HumanVerificationType,
    LabelEvent, LabelId, LabelType, MessageEvent, MessageId, MoreEvents, UserUid,
};
use proton_api_rs::http::Sequence;
use proton_api_rs::log::debug;
use proton_api_rs::{
    captcha_get, http, LoginError, Session, SessionRefreshData, SessionType, TotpSession,
};
use secrecy::{Secret, SecretString};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt::Debug;
use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;

const PROTON_APP_VERSION: &str = "Other";

type Client = http::ureq_client::UReqClient;

/// Create a proton mail backend.
pub fn new_backend() -> Arc<dyn Backend> {
    Arc::new(ProtonBackend {})
}

#[derive(Debug)]
struct ProtonBackend {}

pub const PROTON_BACKEND_NAME: &str = "Proton Mail";
pub const PROTON_BACKEND_NAME_OTHER: &str = "Proton Mail V-Other";

#[derive(Debug)]
struct TaskState {
    last_event_id: Option<EventId>,
    active_folder_ids: HashSet<LabelId>,
}

impl TaskState {
    fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(TaskState {
            last_event_id: None,
            active_folder_ids: HashSet::from([LabelId::inbox()]),
        }))
    }

    fn handle_label_events(&mut self, events: &[LabelEvent]) {
        for event in events {
            match event.action {
                EventAction::Create => {
                    if let Some(label) = event.label.as_ref() {
                        if label.notify == Boolean::True {
                            debug!("New custom folder added to notification list: {}", label.id);
                            self.active_folder_ids.insert(label.id.clone());
                        }
                    }
                }

                EventAction::Update | EventAction::UpdateFlags => {
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

                EventAction::Delete => {
                    debug!("Folder {} deleted", event.id);
                    self.active_folder_ids.remove(&event.id);
                }
            }
        }
    }

    fn should_publish_notification(&self, label_list: &[LabelId]) -> bool {
        for id in label_list {
            if self.active_folder_ids.contains(id) {
                return true;
            }
        }

        false
    }
}

#[derive(Debug)]
struct ProtonAccount {
    email: String,
    client: Client,
    session: Session,
    task_state: Arc<Mutex<TaskState>>,
}

#[derive(Debug)]
struct ProtonTask {
    client: Client,
    email: String,
    session: Session,
    task_state: Arc<Mutex<TaskState>>,
}

#[derive(Debug)]
struct ProtonAuthRefresher {
    email: String,
    uid: Secret<UserUid>,
    token: SecretString,
}

impl ProtonAuthRefresher {
    fn to_json(&self) -> serde_json::Result<serde_json::Value> {
        #[derive(Serialize)]
        struct W<'a> {
            email: &'a str,
            uid: &'a str,
            token: &'a str,
        }

        let data = W {
            email: &self.email,
            uid: self.uid.expose_secret().as_str(),
            token: self.token.expose_secret(),
        };

        serde_json::to_value(data)
    }

    fn from_json(value: serde_json::Value) -> serde_json::Result<Self> {
        #[derive(Deserialize)]
        struct R {
            email: String,
            uid: String,
            token: String,
        }

        let r = serde_json::from_value::<R>(value)?;
        Ok(Self {
            email: r.email,
            uid: Secret::new(UserUid::from(r.uid)),
            token: SecretString::new(r.token),
        })
    }
}

impl ProtonAccount {
    fn new(client: Client, session: Session, email: String) -> Self {
        Self {
            email,
            client,
            session,
            task_state: TaskState::new(),
        }
    }
}

#[derive(Debug)]
struct ProtonAwaitTotp {
    email: String,
    client: Client,
    session: TotpSession,
}

impl Backend for ProtonBackend {
    fn name(&self) -> &str {
        PROTON_BACKEND_NAME
    }

    fn description(&self) -> &str {
        "For Proton accounts (mail.proton.com)"
    }

    fn login(
        &self,
        username: &str,
        password: &SecretString,
        proxy: Option<&Proxy>,
        hv_data: Option<String>,
    ) -> BackendResult<AccountState> {
        debug!("Logging into proton account {username}");
        #[derive(Deserialize)]
        struct HVData {
            hv_type: HumanVerificationType,
            hv_token: String,
        }

        let hv_data = if let Some(hv) = hv_data {
            let hv = serde_json::from_str::<HVData>(&hv)
                .map_err(|e| BackendError::HVDataInvalid(e.into()))?;
            if hv.hv_type != HumanVerificationType::Captcha {
                return Err(BackendError::HVDataInvalid(anyhow!(
                    "Only captcha based human verification is supported"
                )));
            }
            Some(HumanVerificationLoginData {
                hv_type: hv.hv_type,
                token: hv.hv_token,
            })
        } else {
            None
        };

        let client = new_client(proxy)?;

        let login_result = Session::login(username, password, hv_data).do_sync(&client);

        if let Err(LoginError::HumanVerificationRequired(hv)) = &login_result {
            if !hv.methods.contains(&HumanVerificationType::Captcha) {
                return Err(BackendError::Unknown(anyhow!(
                    "Human Verification request, but no supported type available"
                )));
            }

            let html = captcha_get(&hv.token, false)
                .do_sync(&client)
                .map_err(|e| BackendError::Request(anyhow!("Failed to retrieve captcha {e}")))?;

            return Err(BackendError::HVCaptchaRequest(html));
        }

        match login_result? {
            SessionType::Authenticated(s) => Ok(AccountState::LoggedIn(Box::new(
                ProtonAccount::new(client, s, username.to_string()),
            ))),
            SessionType::AwaitingTotp(c) => {
                Ok(AccountState::AwaitingTotp(Box::new(ProtonAwaitTotp {
                    client,
                    session: c,
                    email: username.to_string(),
                })))
            }
        }
    }

    fn check_proxy(&self, proxy: &Proxy) -> BackendResult<()> {
        let client = new_client(Some(proxy))?;
        proton_api_rs::ping().do_sync(&client).map_err(|e| e.into())
    }

    fn auth_refresher_from_config(
        &self,
        value: serde_json::Value,
    ) -> Result<Box<dyn AuthRefresher>, Error> {
        let config = ProtonAuthRefresher::from_json(value).map_err(|e| anyhow!(e))?;
        Ok(Box::new(config))
    }
}

impl Account for ProtonAccount {
    fn new_task(&self) -> Box<dyn CheckTask> {
        Box::new(ProtonTask {
            client: self.client.clone(),
            email: self.email.clone(),
            session: self.session.clone(),
            task_state: self.task_state.clone(),
        })
    }

    fn logout(&mut self) -> BackendResult<()> {
        debug!("Logging out of proton account {}", self.email);
        self.session.logout().do_sync(&self.client)?;
        Ok(())
    }

    fn set_proxy(&mut self, proxy: Option<&Proxy>) -> BackendResult<()> {
        let new_client = new_client(proxy)?;
        self.client = new_client;
        Ok(())
    }

    fn to_refresher(&self) -> Box<dyn AuthRefresher> {
        let refresh_data = self.session.get_refresh_data();
        Box::new(ProtonAuthRefresher {
            email: self.email.clone(),
            uid: refresh_data.user_uid,
            token: refresh_data.token,
        })
    }
}

impl AwaitTotp for ProtonAwaitTotp {
    fn submit_totp(&self, totp: &str) -> Result<Box<dyn Account>, BackendError> {
        match self.session.submit_totp(totp).do_sync(&self.client) {
            Ok(c) => Ok(Box::new(ProtonAccount::new(
                self.client.clone(),
                c,
                self.email.clone(),
            ))),
            Err(e) => Err(e.into()),
        }
    }
}

impl AuthRefresher for ProtonAuthRefresher {
    fn refresh(&self, proxy: Option<&Proxy>) -> Result<AccountState, BackendError> {
        let client = new_client(proxy)?;
        let session = Session::refresh(self.uid.expose_secret(), self.token.expose_secret())
            .do_sync(&client)?;
        Ok(AccountState::LoggedIn(Box::new(ProtonAccount::new(
            client,
            session,
            self.email.clone(),
        ))))
    }

    fn to_json(&self) -> serde_json::Result<serde_json::Value> {
        self.to_json()
    }
}

impl CheckTask for ProtonTask {
    fn email(&self) -> &str {
        &self.email
    }

    fn backend_name(&self) -> &str {
        PROTON_BACKEND_NAME
    }

    fn check(&self, notifier: &mut dyn AccountRefreshedNotifier) -> BackendResult<NewEmailReply> {
        let mut accessor = self.task_state.lock();
        let refresh_data = self.session.get_refresh_data();

        struct RefreshCheck<'a> {
            task: &'a ProtonTask,
            initial_state: SessionRefreshData,
            session: &'a Session,
            notifier: &'a mut dyn AccountRefreshedNotifier,
        }

        impl<'a> Drop for RefreshCheck<'a> {
            fn drop(&mut self) {
                let current_state = self.session.get_refresh_data();
                if current_state != self.initial_state {
                    self.notifier.notify_account_refreshed(self.task);
                }
            }
        }

        let _refresh_checker = RefreshCheck {
            task: self,
            initial_state: refresh_data,
            session: &self.session,
            notifier,
        };

        // First time this code is run, init state.
        if accessor.last_event_id.is_none() {
            let event_id = self.session.get_latest_event().do_sync(&self.client)?;
            accessor.last_event_id = Some(event_id);

            let folders = self
                .session
                .get_labels(LabelType::Folder)
                .do_sync(&self.client)?;
            accessor.active_folder_ids.reserve(folders.len());
            for folder in folders {
                if folder.notify == Boolean::True {
                    accessor.active_folder_ids.insert(folder.id);
                }
            }
            debug!(
                "Account has following list of custom folders: {:?}",
                accessor.active_folder_ids
            )
        }

        let mut result = EventState::new();
        if let Some(mut event_id) = accessor.last_event_id.clone() {
            let mut has_more = MoreEvents::No;
            loop {
                let event = self.session.get_event(&event_id).do_sync(&self.client)?;
                if event.event_id != event_id || has_more == MoreEvents::Yes {
                    if let Some(label_events) = &event.labels {
                        accessor.handle_label_events(label_events)
                    }

                    if let Some(message_events) = &event.messages {
                        result.handle_message_events(message_events, accessor.deref());
                    }

                    event_id = event.event_id;
                    accessor.last_event_id = Some(event_id.clone());
                    has_more = event.more;
                } else {
                    return Ok(result.into());
                }
            }
        } else {
            Err(BackendError::Unknown(anyhow!("Unexpected state")))
        }
    }

    fn to_refresher(&self) -> Box<dyn AuthRefresher> {
        let session_data = self.session.get_refresh_data();
        Box::new(ProtonAuthRefresher {
            email: self.email.clone(),
            uid: session_data.user_uid,
            token: session_data.token,
        })
    }
}

impl From<LoginError> for BackendError {
    fn from(value: LoginError) -> Self {
        match value {
            LoginError::ServerProof(_) => BackendError::Request(anyhow!(value)),
            LoginError::Request(e) => e.into(),
            LoginError::Unsupported2FA(_) => BackendError::Unknown(anyhow!(value)),
            LoginError::SRPProof(_) => BackendError::Unknown(anyhow!(value)),
            _ => BackendError::Unknown(anyhow!("Unhandled Login Error")),
        }
    }
}

impl From<http::Error> for BackendError {
    fn from(value: http::Error) -> Self {
        match value {
            http::Error::API(e) => {
                if e.http_code == 401 {
                    return BackendError::LoggedOut;
                }
                BackendError::API(e.into())
            }
            http::Error::Redirect(_, err) => BackendError::Request(err),
            http::Error::Timeout(err) => BackendError::Timeout(err),
            http::Error::Connection(err) => BackendError::Connection(err),
            http::Error::Request(err) => BackendError::Request(err),
            http::Error::Other(err) => BackendError::Unknown(err),
            http::Error::EncodeOrDecode(err) => BackendError::EncodeOrDecode(err),
        }
    }
}

fn proxy_as_proton_proxy(proxy: &Proxy) -> http::Proxy {
    http::Proxy {
        protocol: match proxy.protocol {
            ProxyProtocol::Https => http::ProxyProtocol::Https,
            ProxyProtocol::Socks5 => http::ProxyProtocol::Socks5,
        },
        auth: proxy.auth.as_ref().map(|a| http::ProxyAuth {
            username: a.username.clone(),
            password: SecretString::new(a.password.clone()),
        }),
        url: proxy.url.clone(),
        port: proxy.port,
    }
}

fn new_client(proxy: Option<&Proxy>) -> Result<Client, BackendError> {
    let mut builder = http::ClientBuilder::new().app_version(PROTON_APP_VERSION);
    if let Some(p) = proxy {
        builder = builder.with_proxy(proxy_as_proton_proxy(p));
    }

    builder
        .connect_timeout(Duration::from_secs(60))
        .request_timeout(Duration::from_secs(3 * 60))
        .build::<Client>()
        .map_err(|e| BackendError::Unknown(anyhow!(e)))
}

struct MsgInfo {
    id: MessageId,
    sender: String,
    subject: String,
}

/// Track the state of a message in a certain event steam so that we can only display a
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

    fn handle_message_events(&mut self, msg_events: &[MessageEvent], state: &TaskState) {
        for msg_event in msg_events {
            match msg_event.action {
                EventAction::Create => {
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
                EventAction::Update | EventAction::UpdateFlags => {
                    if let Some(message) = &msg_event.message {
                        // If message switches to unread state, remove
                        if message.unread == Boolean::False {
                            self.unseen.remove(&message.id);
                        }
                    }
                }
                // Message Deleted, remove from the list.
                EventAction::Delete => {
                    self.unseen.remove(&msg_event.id);
                }
            };
        }
    }

    fn into_new_email_reply(self) -> NewEmailReply {
        if self.unseen.is_empty() {
            return NewEmailReply { emails: vec![] };
        }

        let mut result = Vec::with_capacity(self.unseen.len());

        for msg in self.new_emails {
            if self.unseen.contains(&msg.id) {
                result.push(EmailInfo {
                    sender: msg.sender,
                    subject: msg.subject,
                })
            }
        }

        NewEmailReply { emails: result }
    }
}

impl From<EventState> for NewEmailReply {
    fn from(value: EventState) -> Self {
        value.into_new_email_reply()
    }
}
