use crate::yhm::YhmError;
use tracing::error;
use you_have_mail_common as yhm;
use you_have_mail_common::backend::proton::proton_api::client::ProtonExtension;
use you_have_mail_common::backend::proton::proton_api::requests::Ping;
use you_have_mail_common::secrecy::{ExposeSecret, SecretString};

/// Proxy protocol.
#[derive(uniffi::Enum)]
pub enum Protocol {
    Http,
    Socks5,
}

impl From<yhm::you_have_mail_http::ProxyProtocol> for Protocol {
    fn from(value: yhm::you_have_mail_http::ProxyProtocol) -> Self {
        match value {
            yhm::you_have_mail_http::ProxyProtocol::Http => Self::Http,
            yhm::you_have_mail_http::ProxyProtocol::Socks5 => Self::Socks5,
        }
    }
}

impl From<Protocol> for yhm::you_have_mail_http::ProxyProtocol {
    fn from(value: Protocol) -> Self {
        match value {
            Protocol::Http => yhm::you_have_mail_http::ProxyProtocol::Http,
            Protocol::Socks5 => yhm::you_have_mail_http::ProxyProtocol::Socks5,
        }
    }
}

/// Proxy authentication.
#[derive(uniffi::Record)]
pub struct Auth {
    pub user: String,
    pub password: String,
}

impl From<yhm::you_have_mail_http::ProxyAuth> for Auth {
    fn from(value: yhm::you_have_mail_http::ProxyAuth) -> Self {
        Self {
            user: value.username,
            password: value.password.expose_secret().to_owned(),
        }
    }
}

impl From<Auth> for yhm::you_have_mail_http::ProxyAuth {
    fn from(value: Auth) -> Self {
        yhm::you_have_mail_http::ProxyAuth {
            username: value.user,
            password: SecretString::new(value.password.into()),
        }
    }
}

/// Proxy configuration.
#[derive(uniffi::Record)]
pub struct Proxy {
    /// Protocol
    pub protocol: Protocol,
    /// Host
    pub host: String,
    /// Port
    pub port: u16,
    pub auth: Option<Auth>,
}

impl From<yhm::you_have_mail_http::Proxy> for Proxy {
    fn from(value: yhm::you_have_mail_http::Proxy) -> Self {
        Self {
            protocol: value.protocol.into(),
            host: value.host,
            auth: value.auth.map(Into::into),
            port: value.port,
        }
    }
}

impl From<Proxy> for yhm::you_have_mail_http::Proxy {
    fn from(value: Proxy) -> Self {
        yhm::you_have_mail_http::Proxy {
            protocol: value.protocol.into(),
            host: value.host,
            auth: value.auth.map(Into::into),
            port: value.port,
        }
    }
}

/// Test a proxy by constructing a client and trying to ping a server.
///
/// # Errors
///
/// Returns error if the proxy configuration is invalid or the test failed.
#[uniffi::export]
pub fn test_proxy(proxy: Proxy) -> Result<(), YhmError> {
    let client = yhm::you_have_mail_http::Client::proton_client()
        .with_proxy(proxy.into())
        .build()
        .map_err(|e| {
            error!("Failed to build client with proxy: {e}");
            YhmError::ProxyTest(e.to_string())
        })?;
    client.execute(&Ping {}).map_err(|e| {
        error!("Failed ping server using proxy: {e}");
        YhmError::ProxyTest(e.to_string())
    })?;
    Ok(())
}
