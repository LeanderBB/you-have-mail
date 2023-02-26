//! You have mail implementation for proton mail accounts.

use crate::backend::{Account, AwaitTotp, Backend, BackendError, BackendResult, NewEmailReply};
use crate::AccountState;
use anyhow::anyhow;
use async_trait::async_trait;
use proton_api_rs::domain::{EventId, LabelID, MessageAction, MoreEvents};
use proton_api_rs::{
    Client, ClientBuilder, ClientBuilderError, ClientLoginState, HttpClientError, RequestError,
    TOTPClient,
};
use std::fmt::{Debug, Formatter};

pub fn new_proton_backend(app_version: &str) -> Box<dyn Backend> {
    Box::new(ProtonBackend {
        builder: ClientBuilder::new().app_version(app_version),
    })
}

struct ProtonBackend {
    builder: ClientBuilder,
}

impl Debug for ProtonBackend {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ProtonBackend")
    }
}

struct ProtonAccount {
    client: Option<Client>,
    last_event_id: Option<EventId>,
}

impl ProtonAccount {
    fn new(c: Client) -> Self {
        Self {
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
        "Proton Mail"
    }

    async fn login(&self, username: &str, password: &str) -> BackendResult<AccountState> {
        match self.builder.clone().login(username, password).await? {
            ClientLoginState::Authenticated(c) => {
                Ok(AccountState::LoggedIn(Box::new(ProtonAccount::new(c))))
            }
            ClientLoginState::AwaitingTotp(c) => {
                Ok(AccountState::AwaitingTotp(Box::new(ProtonAwaitTotp {
                    client: c,
                })))
            }
        }
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
}

#[async_trait]
impl AwaitTotp for ProtonAwaitTotp {
    async fn submit_totp(
        mut self: Box<Self>,
        totp: &str,
    ) -> Result<Box<dyn Account>, (Box<dyn AwaitTotp>, BackendError)> {
        match self.client.submit_totp(totp).await {
            Ok(c) => Ok(Box::new(ProtonAccount::new(c))),
            Err((c, e)) => {
                self.client = c;
                Err((self, e.into()))
            }
        }
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
