//! You have mail implementation for proton mail accounts.

use crate::backend::{
    Account, AuthRefresher, AwaitTotp, Backend, BackendError, BackendResult, EmailInfo,
    NewEmailReply,
};
use crate::{AccountState, Proxy, ProxyProtocol};
use anyhow::{anyhow, Error};
use proton_api_rs::domain::{EventId, ExposeSecret, LabelID, MessageAction, MoreEvents, UserUid};
use proton_api_rs::{
    http, AutoAuthRefreshRequestPolicy, AutoRefreshAuthSession, DefaultSessionRequestPolicy,
    LoginError, SessionType, TotpSession,
};
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;

const PROTON_APP_VERSION: &str = "web-mail@5.0.19.5";
const PROTON_APP_VERSION_OTHER: &str = "Other";

type Client = http::ureq_client::UReqClient;
type Session = AutoRefreshAuthSession;

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
    fn new(client: Client, session: Session, email: String, version: &'static str) -> Self {
        Self {
            email,
            client,
            session: Some(session),
            last_event_id: None,
            version,
        }
    }
}

#[derive(Debug)]
struct ProtonAwaitTotp {
    email: String,
    client: Client,
    session: TotpSession<AutoAuthRefreshRequestPolicy<DefaultSessionRequestPolicy>>,
    version: &'static str,
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
            "For Proton accounts (mail.proton.com) - Uses 'Other' as app version"
        }
    }

    fn login(
        &self,
        email: &str,
        password: &str,
        proxy: Option<&Proxy>,
    ) -> BackendResult<AccountState> {
        let client = new_client(proxy, self.version)?;

        match Session::login(&client, email, password)? {
            SessionType::Authenticated(s) => Ok(AccountState::LoggedIn(Box::new(
                ProtonAccount::new(client, s, email.to_string(), self.version),
            ))),
            SessionType::AwaitingTotp(c) => {
                Ok(AccountState::AwaitingTotp(Box::new(ProtonAwaitTotp {
                    version: self.version,
                    client,
                    session: c,
                    email: email.to_string(),
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
        if let Some(session) = &mut self.session {
            if self.last_event_id.is_none() {
                match session.get_latest_event(&self.client) {
                    Err(e) => return (Err(e.into()), false),
                    Ok(event_id) => {
                        self.last_event_id = Some(event_id);
                    }
                }
            }

            let mut result = NewEmailReply { emails: vec![] };
            let mut account_refreshed = false;

            if let Some(event_id) = &mut self.last_event_id {
                let mut has_more = MoreEvents::No;
                loop {
                    match session.get_event(&self.client, event_id) {
                        Err(e) => {
                            account_refreshed = account_refreshed || session.was_auth_refreshed();
                            return (Err(e.into()), account_refreshed);
                        }
                        Ok(event) => {
                            account_refreshed = account_refreshed || session.was_auth_refreshed();
                            if event.event_id != *event_id || has_more == MoreEvents::Yes {
                                if let Some(message_events) = &event.messages {
                                    for msg_event in message_events {
                                        if msg_event.action == MessageAction::Create {
                                            if let Some(message) = &msg_event.message {
                                                if message.labels.contains(&LabelID::inbox()) {
                                                    result.emails.push(EmailInfo {
                                                        subject: message.subject.clone(),
                                                        sender: if let Some(name) =
                                                            &message.sender_name
                                                        {
                                                            name.clone()
                                                        } else {
                                                            message.sender_address.clone()
                                                        },
                                                    })
                                                }
                                            }
                                        }
                                    }
                                }

                                *event_id = event.event_id;
                                has_more = event.more;
                            } else {
                                return (Ok(result), account_refreshed);
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

        let session = Session::refresh(&client, &UserUid::from(self.uid), &self.token)?;
        Ok(AccountState::LoggedIn(Box::new(ProtonAccount::new(
            client,
            session,
            self.email,
            self.version,
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
