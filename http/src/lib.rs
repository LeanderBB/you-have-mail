#![allow(clippy::result_large_err)]
//! Convenience HTTP request handlers that use ureq underneath in order to ensure safe usage
//! when reading the body and reducing boilerplate.

use secrecy::{ExposeSecret, SecretString};
use serde::de::DeserializeOwned;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashMap;
use std::io;
use std::io::Read;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;
pub use ureq;
use ureq::{ErrorKind, Response};
pub use url;
use url::Url;

/// Errors that may arrise during an http request.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// HTTP status error.
    #[error("Http: {0}")]
    Http(u16, Response),
    /// HTTP Transport error.
    #[error("Transport: {0}")]
    Transport(ureq::Transport),
    /// Json serialization or deserialization error.
    #[error("Json Serialization: {0}")]
    Json(#[from] serde_json::Error),
    /// IO Error
    #[error("IO: {0}")]
    IO(#[from] io::Error),
    /// Parsing or manipulation of Urls.
    #[error("Url: {0}")]
    Url(#[from] url::ParseError),
    /// Unexpected use case.
    #[error("Unexpected: {0}")]
    Unexpected(anyhow::Error),
}

impl From<ureq::Error> for Error {
    fn from(value: ureq::Error) -> Self {
        match value {
            ureq::Error::Status(code, response) => Self::Http(code, response),
            ureq::Error::Transport(t) => Self::Transport(t),
        }
    }
}

impl Error {
    /// Whether the current error is a connection error that may indicate there are issues
    /// connecting to the server.
    #[must_use]
    pub fn is_connection_error(&self) -> bool {
        let Self::Transport(err) = self else {
            return false;
        };
        matches!(
            err.kind(),
            ErrorKind::Dns
                | ErrorKind::ConnectionFailed
                | ErrorKind::TooManyRedirects
                | ErrorKind::InvalidProxyUrl
                | ErrorKind::ProxyConnect
        )
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// How to process the response.
pub trait FromResponse {
    /// Result of processing the response.
    type Output;
    /// Process the response from the server.
    ///
    /// This function will only be called if the server did not return an error status.
    ///
    /// # Errors
    /// Should return error if the operation failed.
    fn from_response(response: ureq::Response) -> Result<Self::Output>;
}

/// This response handler does not preform any processing on the response from the server
/// if the request succeeded.
pub struct NoResponse {}

impl FromResponse for NoResponse {
    type Output = ();
    fn from_response(_: ureq::Response) -> Result<Self::Output> {
        Ok(())
    }
}

/// This response handler deserializes the body into a json type `T` from the sever response.
pub struct JsonResponse<T: DeserializeOwned>(PhantomData<T>);

impl<T: DeserializeOwned> FromResponse for JsonResponse<T> {
    type Output = T;
    fn from_response(response: ureq::Response) -> Result<Self::Output> {
        let body = response.into_safe_reader();
        Ok(serde_json::from_reader(body)?)
    }
}

/// This response handler converts the response body from the sever into a string.
pub struct StringResponse {}

impl FromResponse for StringResponse {
    type Output = String;

    fn from_response(response: Response) -> Result<Self::Output> {
        let mut result = String::new();
        response.into_safe_reader().read_to_string(&mut result)?;
        Ok(result)
    }
}

/// HTTP method for the request.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Method {
    Delete,
    Get,
    Patch,
    Post,
    Put,
}

/// Defines an Http Request.
pub trait Request {
    /// How the response should be handled.
    type Response: FromResponse;

    /// Http Method.
    const METHOD: Method;

    /// The relative url of the request without query components.
    fn url(&self) -> String;

    /// Build the request.
    ///
    /// Query parameters and body should be set here.
    ///
    /// # Errors
    /// Returns error if building the operation failed.
    fn build(&self, builder: RequestBuilder) -> Result<RequestBuilder> {
        Ok(builder)
    }
}

pub struct RequestBuilder {
    request: ureq::Request,
    body: Option<Vec<u8>>,
}

impl RequestBuilder {
    fn new(request: ureq::Request) -> Self {
        Self {
            request,
            body: None,
        }
    }
    /// Set a header with `key` and `value`.
    #[must_use]
    pub fn header(mut self, key: impl AsRef<str>, value: impl AsRef<str>) -> Self {
        self.request = self.request.set(key.as_ref(), value.as_ref());
        self
    }

    /// Set bearer authentication `token`.
    #[must_use]
    pub fn bearer_token(self, token: impl AsRef<str>) -> Self {
        self.header("authorization", format!("Bearer {}", token.as_ref()))
    }

    /// Set the body as a collection of bytes.
    #[must_use]
    pub fn bytes(mut self, bytes: Vec<u8>) -> Self {
        self.body = Some(bytes);
        self
    }

    /// Set the body as a serialized json object.
    ///
    /// # Panics
    /// Will panic if the type can not be serialized to json.
    #[must_use]
    pub fn json(self, value: impl Serialize) -> Self {
        let bytes = serde_json::to_vec(&value).expect("Failed to serialize json");
        self.json_bytes(bytes)
    }

    #[must_use]
    fn json_bytes(mut self, bytes: Vec<u8>) -> Self {
        self.body = Some(bytes);
        self.header("Content-Type", "application/json")
    }

    /// Set a query parameter with `key` and `value`.
    #[must_use]
    pub fn query(mut self, key: impl AsRef<str>, value: impl AsRef<str>) -> Self {
        self.request = self.request.query(key.as_ref(), value.as_ref());
        self
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum ProxyProtocol {
    Https,
    Socks5,
}

/// Proxy authentication information.
#[derive(Debug, Clone, Deserialize)]
pub struct ProxyAuth {
    /// Username.
    pub username: String,
    /// User password.
    pub password: SecretString,
}

impl PartialEq for ProxyAuth {
    fn eq(&self, other: &Self) -> bool {
        self.username == other.username
            && self.password.expose_secret() == other.password.expose_secret()
    }
}

impl Eq for ProxyAuth {}

impl Serialize for ProxyAuth {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("ProxyAuth", 2)?;
        state.serialize_field("username", self.username.as_str())?;
        state.serialize_field("password", self.password.expose_secret().as_str())?;
        state.end()
    }
}

/// HTTP proxy configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct Proxy {
    /// Protocol of the proxy.
    pub protocol: ProxyProtocol,
    /// Optional proxy authentication.
    pub auth: Option<ProxyAuth>,
    /// Url of the proxy.
    pub host: String,
    /// Port of the proxy.
    pub port: u16,
}

impl Proxy {
    /// Convert the proxy configuration into a usable url.
    ///
    /// # Errors
    /// Returns error if the generated url is not valid.
    pub fn to_url(&self) -> Result<Url> {
        let protocol = match self.protocol {
            ProxyProtocol::Https => "https",
            ProxyProtocol::Socks5 => "socks5",
        };

        let auth = if let Some(auth) = &self.auth {
            format!("{}:{}@", auth.username, auth.password.expose_secret())
        } else {
            String::new()
        };

        Ok(Url::parse(&format!(
            "{protocol}://{auth}{}:{}",
            self.host.as_str(),
            self.port
        ))?)
    }
}

/// Http client builder.
#[derive(Debug)]
pub struct ClientBuilder {
    base_url: Url,
    request_timeout: Option<Duration>,
    connect_timeout: Option<Duration>,
    user_agent: String,
    proxy: Option<Proxy>,
    debug: bool,
    allow_http: bool,
    default_headers: HashMap<String, String>,
}

impl ClientBuilder {
    fn new(base_url: Url) -> Self {
        Self {
            user_agent: "NoClient/0.1.0".to_string(),
            base_url,
            request_timeout: None,
            connect_timeout: None,
            proxy: None,
            debug: false,
            allow_http: false,
            default_headers: HashMap::new(),
        }
    }

    /// Set the user agent to be submitted with every request.
    #[must_use]
    pub fn user_agent(mut self, agent: &str) -> Self {
        self.user_agent = agent.to_string();
        self
    }

    /// Set the full request timeout. By default there is no timeout.
    #[must_use]
    pub fn request_timeout(mut self, duration: Duration) -> Self {
        self.request_timeout = Some(duration);
        self
    }

    /// Set the connection timeout. By default there is no timeout.
    #[must_use]
    pub fn connect_timeout(mut self, duration: Duration) -> Self {
        self.connect_timeout = Some(duration);
        self
    }

    /// Specify proxy URL for the builder.
    #[must_use]
    pub fn with_proxy(mut self, proxy: Proxy) -> Self {
        self.proxy = Some(proxy);
        self
    }

    /// Allow http request
    #[must_use]
    pub fn allow_http(mut self) -> Self {
        self.allow_http = true;
        self
    }

    /// Enable request debugging.
    #[must_use]
    pub fn debug(mut self) -> Self {
        self.debug = true;
        self
    }

    /// Set a header with `key` and `value`.
    #[must_use]
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.default_headers.insert(key.into(), value.into());
        self
    }

    /// Create the client.
    ///
    /// # Errors
    /// Returns error if the construction failed.
    pub fn build(self) -> Result<Arc<Client>> {
        let mut builder = ureq::AgentBuilder::new();

        if let Some(d) = self.request_timeout {
            builder = builder.timeout(d);
        }

        if let Some(d) = self.connect_timeout {
            builder = builder.timeout_connect(d);
        }

        if let Some(proxy) = &self.proxy {
            let proxy = ureq::Proxy::new(proxy.to_url()?.as_str())?;
            builder = builder.proxy(proxy);
        }

        if !self.allow_http {
            builder = builder.https_only(true);
        }

        let agent = builder
            .user_agent(&self.user_agent)
            .max_idle_connections(0)
            .max_idle_connections_per_host(0)
            .build();

        Ok(Arc::new(Client {
            agent,
            base_url: self.base_url,
            default_headers: self.default_headers,
            proxy: self.proxy,
        }))
    }
}

/// HTTP Client on which to execute requests.
///
/// All request executed on this client will be appended to the base url.
pub struct Client {
    agent: ureq::Agent,
    base_url: Url,
    default_headers: HashMap<String, String>,
    proxy: Option<Proxy>,
}

impl Client {
    /// Create a new builder with the given `base_url`.
    #[must_use]
    pub fn builder(base_url: Url) -> ClientBuilder {
        ClientBuilder::new(base_url)
    }

    /// The base url in use by the client.
    #[must_use]
    pub fn base_url(&self) -> &Url {
        &self.base_url
    }

    /// The proxy configuration in use by the client
    #[must_use]
    pub fn proxy(&self) -> Option<&Proxy> {
        self.proxy.as_ref()
    }

    /// Execute the request and return the result.
    ///
    /// This is just a thin wrapper around [`ureq::Request`] that sets default headers
    /// and executes the correct function depending on whether the request has a body or not.
    ///
    /// # Errors
    /// Returns an error if the request construction, execution or response handling failed.
    pub fn execute<R: Request>(
        &self,
        request: &R,
    ) -> Result<<R::Response as FromResponse>::Output> {
        let url = self.base_url.join(&request.url())?;
        let ureq_request = match R::METHOD {
            Method::Get => self.agent.get(url.as_str()),
            Method::Put => self.agent.put(url.as_str()),
            Method::Post => self.agent.post(url.as_str()),
            Method::Delete => self.agent.delete(url.as_str()),
            Method::Patch => self.agent.patch(url.as_str()),
        };

        let mut builder = RequestBuilder::new(ureq_request);

        for (key, value) in &self.default_headers {
            builder = builder.header(key, value);
        }

        let builder = request.build(builder)?;

        let ureq_response = if let Some(body) = builder.body {
            builder.request.send_bytes(body.as_ref())?
        } else {
            builder.request.call()?
        };

        R::Response::from_response(ureq_response)
    }
}

/// Extension trait to read the body with safe upper limit.
pub trait ExtSafeResponse {
    /// Create a safe reader that reads up to a maximum number of bytes from the server.
    fn into_safe_reader(self) -> impl Read;
}

const MAX_BYTES_FROM_RESPONSE: u64 = 10_000_000;

impl ExtSafeResponse for ureq::Response {
    fn into_safe_reader(self) -> impl Read {
        self.into_reader().take(MAX_BYTES_FROM_RESPONSE)
    }
}

#[test]
fn proxy_config_generates_valid_url() {
    let host = "foo.bar.com";
    let port = 22;
    // Https configuration.
    let proxy = Proxy {
        protocol: ProxyProtocol::Https,
        auth: None,
        host: host.to_owned(),
        port,
    };

    let url = proxy.to_url().unwrap();
    assert_eq!(url.scheme(), "https");
    assert_eq!(url.host_str().unwrap(), host);
    assert_eq!(url.port().unwrap(), port);

    // Socks5 configuration.
    let proxy = Proxy {
        protocol: ProxyProtocol::Socks5,
        auth: None,
        host: host.to_owned(),
        port,
    };

    let url = proxy.to_url().unwrap();
    assert_eq!(url.scheme(), "socks5");
    assert_eq!(url.host_str().unwrap(), host);
    assert_eq!(url.port().unwrap(), port);

    // With Authentication
    let proxy = Proxy {
        protocol: ProxyProtocol::Socks5,
        auth: Some(ProxyAuth {
            username: "Foo".to_string(),
            password: SecretString::new("bar".to_string()),
        }),
        host: host.to_owned(),
        port,
    };

    let url = proxy.to_url().unwrap();
    assert_eq!(url.scheme(), "socks5");
    assert_eq!(url.host_str().unwrap(), host);
    assert_eq!(url.port().unwrap(), port);
    assert_eq!(url.password().unwrap(), "bar");
    assert_eq!(url.username(), "Foo");
}

#[test]
fn proxy_serialize_deserialize() {
    let host = "foo.bar.com";
    let proxy = Proxy {
        protocol: ProxyProtocol::Socks5,
        auth: Some(ProxyAuth {
            username: "Foo".to_string(),
            password: SecretString::new("bar".to_string()),
        }),
        host: host.to_owned(),
        port: 1024,
    };

    let serialized = serde_json::to_vec(&proxy).unwrap();
    let derserialized = serde_json::from_slice::<Proxy>(&serialized).unwrap();
    assert_eq!(proxy, derserialized);
}
