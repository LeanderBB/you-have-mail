//! You have mail implementation for proton mail accounts.

use crate::backend::{
    Account, AuthRefresher, AwaitTotp, Backend, BackendError, BackendResult, NewEmailReply,
};
use crate::AccountState;
use anyhow::{anyhow, Error};
use async_trait::async_trait;
use proton_api_rs::domain::{EventId, ExposeSecret, LabelID, MessageAction, MoreEvents, UserUid};
use proton_api_rs::{
    Client, ClientBuilder, ClientBuilderError, ClientLoginState, HttpClientError, RequestError,
    TOTPClient,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

/// Create a proton mail backend.
pub fn new_backend(app_version: &str) -> Arc<dyn Backend> {
    Arc::new(ProtonBackend {
        builder: ClientBuilder::new().app_version(app_version),
    })
}

struct ProtonBackend {
    builder: ClientBuilder,
}

const PROTON_BACKEND_NAME: &str = "Proton Mail";

impl Debug for ProtonBackend {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ProtonBackend")
    }
}

struct ProtonAccount {
    email: String,
    client: Option<Client>,
    last_event_id: Option<EventId>,
}

struct ProtonAuthRefresher {
    builder: ClientBuilder,
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

impl Debug for ProtonAccount {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ProtonAccount")
    }
}

struct ProtonAwaitTotp {
    email: String,
    client: TOTPClient,
}

impl Debug for ProtonAwaitTotp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ProtonAwaitTotp")
    }
}

#[async_trait]
impl Backend for ProtonBackend {
    fn name(&self) -> &str {
        PROTON_BACKEND_NAME
    }

    async fn login(&self, email: &str, password: &str) -> BackendResult<AccountState> {
        match self.builder.clone().login(email, password).await? {
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

    fn auth_refresher_from_config(&self, value: Value) -> Result<Box<dyn AuthRefresher>, Error> {
        let config =
            serde_json::from_value::<ProtonAuthRefresherInfo>(value).map_err(|e| anyhow!(e))?;
        Ok(Box::new(ProtonAuthRefresher {
            builder: self.builder.clone(),
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
                let event_id = client.get_latest_event_id().await?;
                self.last_event_id = Some(event_id);
            }

            let mut result = NewEmailReply { count: 0 };

            if let Some(event_id) = &mut self.last_event_id {
                let mut has_more = MoreEvents::No;
                loop {
                    let event = client.get_event(event_id).await?;
                    if event.event_id != *event_id || has_more == MoreEvents::Yes {
                        if let Some(message_events) = &event.messages {
                            for msg_event in message_events {
                                if msg_event.action == MessageAction::Create
                                    && msg_event.message.labels.contains(&LabelID::inbox())
                                {
                                    result.count += 1
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
    async fn refresh(self: Box<Self>) -> Result<AccountState, BackendError> {
        let client = self
            .builder
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
                HttpClientError::Redirect(_) => BackendError::Unknown(anyhow!(e)),
                HttpClientError::Timeout | HttpClientError::Connection => BackendError::Offline,
                _ => BackendError::Unknown(anyhow!(e)),
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
