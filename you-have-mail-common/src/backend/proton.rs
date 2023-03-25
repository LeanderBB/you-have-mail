//! You have mail implementation for proton mail accounts.

use crate::backend::{
    Account, AuthRefresher, AwaitTotp, Backend, BackendError, BackendResult, NewEmailReply,
};
use crate::{AccountState, Proxy, ProxyProtocol};
use anyhow::{anyhow, Error};
use async_trait::async_trait;
use proton_api_rs::domain::{EventId, ExposeSecret, LabelID, MessageAction, MoreEvents, UserUid};
use proton_api_rs::log::{debug, error};
use proton_api_rs::{
    Client, ClientBuilder, ClientBuilderError, ClientLoginState, HttpClientError, RequestError,
    TOTPClient,
};
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;

const PROTON_APP_VERSION: &str = "web-mail@5.0.17.9";

/// Create a proton mail backend.
pub fn new_backend() -> Arc<dyn Backend> {
    Arc::new(ProtonBackend {})
}

#[derive(Debug)]
struct ProtonBackend {}

const PROTON_BACKEND_NAME: &str = "Proton Mail";

#[derive(Debug)]
struct ProtonAccount {
    email: String,
    client: Option<Client>,
    last_event_id: Option<EventId>,
}

#[derive(Debug)]
struct ProtonAuthRefresher {
    email: String,
    uid: String,
    token: String,
}

#[derive(Deserialize)]
struct ProtonAuthRefresherInfo {
    email: String,
    uid: String,
    token: String,
}

#[derive(Serialize)]
struct ProtonAuthRefresherInfoRead<'a> {
    email: &'a str,
    uid: &'a str,
    token: &'a str,
}

impl ProtonAccount {
    fn new(c: Client, email: String) -> Self {
        Self {
            email,
            client: Some(c),
            last_event_id: None,
        }
    }
}

#[derive(Debug)]
struct ProtonAwaitTotp {
    email: String,
    client: TOTPClient,
}

macro_rules! try_request {
    ($request:expr, $refresh:expr) => {{
        let r = $request.await;
        match r {
            Ok(t) => Ok(t),
            Err(e) => {
                if let RequestError::API(api_err) = &e {
                    if api_err.http_code == 401 {
                        debug!("Proton account expired, attempting refresh");
                        // Unauthorized, try to refresh once
                        if let Err(e) = $refresh.await {
                            error!("Proton account expired, refresh failed {e}");
                            Err(e)
                        } else {
                            $request.await
                        }
                    } else {
                        Err(e)
                    }
                } else {
                    Err(e)
                }
            }
        }
    }};
}

#[async_trait]
impl Backend for ProtonBackend {
    fn name(&self) -> &str {
        PROTON_BACKEND_NAME
    }

    fn description(&self) -> &str {
        "For Proton accounts (mail.proton.com)"
    }

    async fn login<'a>(
        &self,
        email: &str,
        password: &str,
        proxy: Option<&'a Proxy>,
    ) -> BackendResult<AccountState> {
        match new_client_builder(proxy).login(email, password).await? {
            ClientLoginState::Authenticated(c) => Ok(AccountState::LoggedIn(Box::new(
                ProtonAccount::new(c, email.to_string()),
            ))),
            ClientLoginState::AwaitingTotp(c) => {
                Ok(AccountState::AwaitingTotp(Box::new(ProtonAwaitTotp {
                    client: c,
                    email: email.to_string(),
                })))
            }
        }
    }

    async fn check_proxy(&self, proxy: &Proxy) -> BackendResult<()> {
        return new_client_builder(Some(proxy))
            .ping()
            .await
            .map_err(|e| e.into());
    }

    fn auth_refresher_from_config(&self, value: Value) -> Result<Box<dyn AuthRefresher>, Error> {
        let config =
            serde_json::from_value::<ProtonAuthRefresherInfo>(value).map_err(|e| anyhow!(e))?;
        Ok(Box::new(ProtonAuthRefresher {
            email: config.email,
            uid: config.uid,
            token: config.token,
        }))
    }
}

#[async_trait]
impl Account for ProtonAccount {
    async fn check(&mut self) -> BackendResult<NewEmailReply> {
        if let Some(client) = &mut self.client {
            if self.last_event_id.is_none() {
                let event_id =
                    try_request!({ client.get_latest_event_id() }, { client.refresh_auth() })?;
                self.last_event_id = Some(event_id);
            }

            let mut result = NewEmailReply { count: 0 };

            if let Some(event_id) = &mut self.last_event_id {
                let mut has_more = MoreEvents::No;
                loop {
                    let event =
                        try_request!({ client.get_event(event_id) }, { client.refresh_auth() })?;
                    if event.event_id != *event_id || has_more == MoreEvents::Yes {
                        if let Some(message_events) = &event.messages {
                            for msg_event in message_events {
                                if msg_event.action == MessageAction::Create {
                                    if let Some(message) = &msg_event.message {
                                        if message.labels.contains(&LabelID::inbox()) {
                                            result.count += 1
                                        }
                                    }
                                }
                            }
                        }

                        *event_id = event.event_id;
                        has_more = event.more;
                    } else {
                        return Ok(result);
                    }
                }
            }
        }

        Err(BackendError::Unknown(anyhow!("Client is no longer active")))
    }

    async fn logout(&mut self) -> BackendResult<()> {
        if let Some(client) = self.client.take() {
            if let Err((c, err)) = client.logout().await {
                self.client = Some(c);
                return Err(err.into());
            }
        }
        Ok(())
    }

    async fn set_proxy<'a>(&mut self, proxy: Option<&'a Proxy>) -> BackendResult<()> {
        if let Some(client) = &mut self.client {
            let new_client = new_client_builder(proxy).with_client_auth(client)?;
            *client = new_client;
            Ok(())
        } else {
            Err(BackendError::Unknown(anyhow!("Client is no longer active")))
        }
    }

    fn auth_refresher_config(&self) -> Result<Value, Error> {
        let Some(client) = &self.client else {
            return Err(anyhow!("invalid state"));
        };
        let info = ProtonAuthRefresherInfoRead {
            email: &self.email,
            uid: client.user_uid().expose_secret().as_str(),
            token: client.user_refresh_token().expose_secret().as_str(),
        };
        let value = serde_json::to_value(&info).map_err(|e| anyhow!(e))?;
        Ok(value)
    }
}

#[async_trait]
impl AwaitTotp for ProtonAwaitTotp {
    async fn submit_totp(
        mut self: Box<Self>,
        totp: &str,
    ) -> Result<Box<dyn Account>, (Box<dyn AwaitTotp>, BackendError)> {
        match self.client.submit_totp(totp).await {
            Ok(c) => Ok(Box::new(ProtonAccount::new(c, self.email))),
            Err((c, e)) => {
                self.client = c;
                Err((self, e.into()))
            }
        }
    }
}

#[async_trait]
impl AuthRefresher for ProtonAuthRefresher {
    async fn refresh<'a>(
        self: Box<Self>,
        proxy: Option<&'a Proxy>,
    ) -> Result<AccountState, BackendError> {
        let client = new_client_builder(proxy)
            .with_token(&UserUid::from(self.uid), &self.token)
            .await?;
        Ok(AccountState::LoggedIn(Box::new(ProtonAccount::new(
            client, self.email,
        ))))
    }
}

impl From<ClientBuilderError> for BackendError {
    fn from(value: ClientBuilderError) -> Self {
        match value {
            ClientBuilderError::ServerProof(_) => BackendError::Request(anyhow!(value)),
            ClientBuilderError::Request(e) => e.into(),
            ClientBuilderError::Unsupported2FA(_) => BackendError::Unknown(anyhow!(value)),
            ClientBuilderError::SRPProof(_) => BackendError::Unknown(anyhow!(value)),
        }
    }
}

impl From<RequestError> for BackendError {
    fn from(value: RequestError) -> Self {
        match value {
            RequestError::HttpClient(e) => match e {
                HttpClientError::Redirect(_, err) => BackendError::Request(err),
                HttpClientError::Timeout(err) => BackendError::Request(err),
                HttpClientError::Request(err) => BackendError::Request(err),
                HttpClientError::Connection(err) => BackendError::Offline(err),
                HttpClientError::Body(err) => BackendError::Request(err),
                HttpClientError::Other(err) => BackendError::Request(err),
            },
            RequestError::API(e) => {
                if e.http_code == 401 {
                    return BackendError::LoggedOut;
                }
                BackendError::Request(anyhow!(e))
            }
            RequestError::JSON(e) => BackendError::Request(anyhow!(e)),
            RequestError::Other(e) => BackendError::Unknown(anyhow!(e)),
        }
    }
}

fn proxy_as_proton_proxy(proxy: &Proxy) -> proton_api_rs::Proxy {
    proton_api_rs::Proxy {
        protocol: match proxy.protocol {
            ProxyProtocol::Https => proton_api_rs::ProxyProtocol::Https,
            ProxyProtocol::Socks5 => proton_api_rs::ProxyProtocol::Socks5,
        },
        auth: proxy.auth.as_ref().map(|a| proton_api_rs::ProxyAuth {
            username: a.username.clone(),
            password: SecretString::new(a.password.clone()),
        }),
        url: proxy.url.clone(),
        port: proxy.port,
    }
}

fn new_client_builder(proxy: Option<&Proxy>) -> ClientBuilder {
    let mut builder = ClientBuilder::new().app_version(PROTON_APP_VERSION);
    if let Some(p) = proxy {
        builder = builder.with_proxy(proxy_as_proton_proxy(p));
    }

    builder
        .connect_timeout(Duration::from_secs(60))
        .request_timeout(Duration::from_secs(3 * 60))
}
