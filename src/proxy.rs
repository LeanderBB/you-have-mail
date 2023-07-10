use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum ProxyProtocol {
    Https,
    Socks5,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProxyAuth {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Proxy {
    pub protocol: ProxyProtocol,
    pub auth: Option<ProxyAuth>,
    pub url: String,
    pub port: u16,
}
