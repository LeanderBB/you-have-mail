//! You have mail implementation for proton mail accounts.

use crate::backend::{
    Account, AuthRefresher, AwaitTotp, Backend, BackendError, BackendResult, EmailInfo,
    NewEmailReply,
};
use crate::{AccountState, Proxy, ProxyProtocol};
use anyhow::{anyhow, Error};
use proton_api_rs::domain::{
    Boolean, EventId, ExposeSecret, HumanVerificationLoginData, HumanVerificationType, LabelID,
    MessageAction, MessageEvent, MessageId, MoreEvents, UserUid,
};
use proton_api_rs::log::info;
use proton_api_rs::{
    captcha_get, http, LoginError, OnAuthRefreshed, Session, SessionType, TotpSession,
};
use secrecy::{Secret, SecretString};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::fmt::Debug;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

const PROTON_APP_VERSION: &str = "web-mail@5.0.19.5";
const PROTON_APP_VERSION_OTHER: &str = "Other";

type Client = http::ureq_client::UReqClient;

/// Create a proton mail backend.
pub fn new_backend() -> Arc<dyn Backend> {
    Arc::new(ProtonBackend {
        version: PROTON_APP_VERSION,
    })
}

/// Create a proton mail backend.
pub fn new_backend_version_other() -> Arc<dyn Backend> {
    Arc::new(ProtonBackend {
        version: PROTON_APP_VERSION_OTHER,
    })
}

#[derive(Debug)]
struct ProtonBackend {
    version: &'static str,
}

const PROTON_BACKEND_NAME: &str = "Proton Mail";
const PROTON_BACKEND_NAME_OTHER: &str = "Proton Mail V-Other";

#[derive(Debug)]
struct ProtonAccount {
    email: String,
    client: Client,
    session: Option<Session>,
    last_event_id: Option<EventId>,
    version: &'static str,
    auth_refresh_checker: AuthRefreshChecker,
}

#[derive(Debug)]
struct ProtonAuthRefresher {
    email: String,
    uid: String,
    token: String,
    version: &'static str,
}

#[derive(Deserialize)]
struct ProtonAuthRefresherInfo {
    email: String,
    uid: String,
    token: String,
    is_other_version: Option<bool>,
}

#[derive(Serialize)]
struct ProtonAuthRefresherInfoRead<'a> {
    email: &'a str,
    uid: &'a str,
    token: &'a str,
    is_other_version: bool,
}

impl ProtonAccount {
    fn new(
        client: Client,
        session: Session,
        email: String,
        version: &'static str,
        auth_refresh_checker: AuthRefreshChecker,
    ) -> Self {
        Self {
            email,
            client,
            session: Some(session),
            last_event_id: None,
            version,
            auth_refresh_checker,
        }
    }
}

#[derive(Debug)]
struct ProtonAwaitTotp {
    email: String,
    client: Client,
    session: TotpSession,
    auth_refresh_checker: AuthRefreshChecker,
    version: &'static str,
}

#[derive(Debug)]
struct AuthRefreshChecker {
    value: Arc<AtomicBool>,
}

impl AuthRefreshChecker {
    fn new() -> Self {
        Self {
            value: Arc::new(AtomicBool::new(false)),
        }
    }

    fn reset(&self) {
        self.value.store(false, Ordering::SeqCst)
    }

    fn value(&self) -> bool {
        self.value.load(Ordering::SeqCst)
    }

    fn to_on_auth_refreshed(&self) -> Box<dyn OnAuthRefreshed> {
        Box::new(AuthRefresherCheckerCB {
            value: self.value.clone(),
        })
    }
}

struct AuthRefresherCheckerCB {
    value: Arc<AtomicBool>,
}

impl OnAuthRefreshed for AuthRefresherCheckerCB {
    fn on_auth_refreshed(&self, _: &Secret<UserUid>, _: &proton_api_rs::domain::SecretString) {
        self.value.store(true, Ordering::SeqCst);
    }
}

impl Backend for ProtonBackend {
    fn name(&self) -> &str {
        if self.version != PROTON_APP_VERSION_OTHER {
            PROTON_BACKEND_NAME
        } else {
            PROTON_BACKEND_NAME_OTHER
        }
    }

    fn description(&self) -> &str {
        if self.version != PROTON_APP_VERSION_OTHER {
            "For Proton accounts (mail.proton.com)"
        } else {
            "For Proton accounts (mail.proton.com) - Bypasses Captcha Request"
        }
    }

    fn login(
        &self,
        username: &str,
        password: &str,
        proxy: Option<&Proxy>,
        hv_data: Option<String>,
    ) -> BackendResult<AccountState> {
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

        let client = new_client(proxy, self.version)?;

        let auth_refresh_checker = AuthRefreshChecker::new();

        let login_result = Session::login(
            &client,
            username,
            password,
            hv_data,
            Some(auth_refresh_checker.to_on_auth_refreshed()),
        );

        if let Err(LoginError::HumanVerificationRequired(hv)) = &login_result {
            if !hv.methods.contains(&HumanVerificationType::Captcha) {
                return Err(BackendError::Unknown(anyhow!(
                    "Human Verification request, but no supported type available"
                )));
            }

            let html = captcha_get(&client, &hv.token, false)
                .map_err(|e| BackendError::Request(anyhow!("Failed to retrieve captcha {e}")))?;

            return Err(BackendError::HVCaptchaRequest(html));
        }

        match login_result? {
            SessionType::Authenticated(s) => {
                Ok(AccountState::LoggedIn(Box::new(ProtonAccount::new(
                    client,
                    s,
                    username.to_string(),
                    self.version,
                    auth_refresh_checker,
                ))))
            }
            SessionType::AwaitingTotp(c) => {
                Ok(AccountState::AwaitingTotp(Box::new(ProtonAwaitTotp {
                    version: self.version,
                    client,
                    session: c,
                    email: username.to_string(),
                    auth_refresh_checker,
                })))
            }
        }
    }

    fn check_proxy(&self, proxy: &Proxy) -> BackendResult<()> {
        let client = new_client(Some(proxy), self.version)?;
        proton_api_rs::ping(&client).map_err(|e| e.into())
    }

    fn auth_refresher_from_config(&self, value: Value) -> Result<Box<dyn AuthRefresher>, Error> {
        let config =
            serde_json::from_value::<ProtonAuthRefresherInfo>(value).map_err(|e| anyhow!(e))?;

        let version = if config.is_other_version.is_none() || !config.is_other_version.unwrap() {
            PROTON_APP_VERSION
        } else {
            PROTON_APP_VERSION_OTHER
        };

        Ok(Box::new(ProtonAuthRefresher {
            email: config.email,
            uid: config.uid,
            token: config.token,
            version,
        }))
    }
}

impl Account for ProtonAccount {
    fn check(&mut self) -> (BackendResult<NewEmailReply>, bool) {
        self.auth_refresh_checker.reset();
        if let Some(session) = &mut self.session {
            if self.last_event_id.is_none() {
                match session.get_latest_event(&self.client) {
                    Err(e) => return (Err(e.into()), self.auth_refresh_checker.value()),
                    Ok(event_id) => {
                        self.last_event_id = Some(event_id);
                    }
                }
            }

            let mut result = EventState::new();
            if let Some(event_id) = &mut self.last_event_id {
                let mut has_more = MoreEvents::No;
                loop {
                    match session.get_event(&self.client, event_id) {
                        Err(e) => {
                            return (Err(e.into()), self.auth_refresh_checker.value());
                        }
                        Ok(event) => {
                            if event.event_id != *event_id || has_more == MoreEvents::Yes {
                                if let Some(message_events) = &event.messages {
                                    result.handle_message_events(message_events);
                                }

                                *event_id = event.event_id;
                                has_more = event.more;
                            } else {
                                return (Ok(result.into()), self.auth_refresh_checker.value());
                            }
                        }
                    }
                }
            }
        }

        (
            Err(BackendError::Unknown(anyhow!("Client is no longer active"))),
            false,
        )
    }

    fn logout(&mut self) -> BackendResult<()> {
        if let Some(session) = self.session.take() {
            if let Err(err) = session.logout(&self.client) {
                self.session = Some(session);
                return Err(err.into());
            }
        }
        Ok(())
    }

    fn set_proxy(&mut self, proxy: Option<&Proxy>) -> BackendResult<()> {
        let new_client = new_client(proxy, self.version)?;
        self.client = new_client;
        Ok(())
    }

    fn auth_refresher_config(&self) -> Result<Value, Error> {
        let Some(session) = &self.session else {
            return Err(anyhow!("invalid state"));
        };
        let refresh_data = session.get_refresh_data();
        let info = ProtonAuthRefresherInfoRead {
            email: &self.email,
            uid: refresh_data.user_uid.expose_secret().as_str(),
            token: refresh_data.token.expose_secret().as_str(),
            is_other_version: self.version == PROTON_APP_VERSION_OTHER,
        };

        serde_json::to_value(info).map_err(|e| anyhow!(e))
    }
}

impl AwaitTotp for ProtonAwaitTotp {
    fn submit_totp(
        mut self: Box<Self>,
        totp: &str,
    ) -> Result<Box<dyn Account>, (Box<dyn AwaitTotp>, BackendError)> {
        match self.session.submit_totp(&self.client, totp) {
            Ok(c) => Ok(Box::new(ProtonAccount::new(
                self.client,
                c,
                self.email,
                self.version,
                self.auth_refresh_checker,
            ))),
            Err((c, e)) => {
                self.session = c;
                Err((self, e.into()))
            }
        }
    }
}

impl AuthRefresher for ProtonAuthRefresher {
    fn refresh(self: Box<Self>, proxy: Option<&Proxy>) -> Result<AccountState, BackendError> {
        let client = new_client(proxy, self.version)?;
        let auth_refreshed_checker = AuthRefreshChecker::new();
        let session = Session::refresh(
            &client,
            &UserUid::from(self.uid),
            &self.token,
            Some(auth_refreshed_checker.to_on_auth_refreshed()),
        )?;
        Ok(AccountState::LoggedIn(Box::new(ProtonAccount::new(
            client,
            session,
            self.email,
            self.version,
            auth_refreshed_checker,
        ))))
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

fn new_client(proxy: Option<&Proxy>, version: &'static str) -> Result<Client, BackendError> {
    let mut builder = http::ClientBuilder::new().app_version(version);
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

    fn handle_message_events(&mut self, msg_events: &[MessageEvent]) {
        let inbox_label = LabelID::inbox();

        for msg_event in msg_events {
            match msg_event.action {
                MessageAction::Create => {
                    if let Some(message) = &msg_event.message {
                        // If the newly created message is not unread, it must have been read
                        // already.
                        if message.unread == Boolean::False {
                            return;
                        }

                        // Check if the message has arrived in the inbox.
                        if message.labels.contains(&inbox_label) {
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
                MessageAction::Update | MessageAction::UpdateFlags => {
                    if let Some(message) = &msg_event.message {
                        // If message switches to unread state, remove
                        if message.unread == Boolean::False
                            || !message.labels.contains(&inbox_label)
                        {
                            info!(
                                "message removed {} {}",
                                message.unread == Boolean::False,
                                !message.labels.contains(&inbox_label)
                            );
                            self.unseen.remove(&message.id);
                        }
                    }
                }
                // Message Deleted, remove from the list.
                MessageAction::Delete => {
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
