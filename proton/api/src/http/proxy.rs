use crate::domain::SecretString;
use secrecy::ExposeSecret;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ProxyProtocol {
    Https,
    Socks5,
}

#[derive(Debug, Clone)]
pub struct ProxyAuth {
    pub username: String,
    pub password: SecretString,
}

#[derive(Debug, Clone)]
pub struct Proxy {
    pub protocol: ProxyProtocol,
    pub auth: Option<ProxyAuth>,
    pub url: String,
    pub port: u16,
}

impl Proxy {
    pub fn as_url(&self) -> String {
        let protocol = match self.protocol {
            ProxyProtocol::Https => "https",
            ProxyProtocol::Socks5 => "socks5",
        };

        let auth = if let Some(auth) = &self.auth {
            format!("{}:{}@", auth.username, auth.password.expose_secret())
        } else {
            String::new()
        };

        format!("{protocol}://{auth}{}:{}", self.url, self.port)
    }
}
