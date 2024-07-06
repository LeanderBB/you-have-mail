use crate::http::{Proxy, RequestData, Result, DEFAULT_APP_VERSION, DEFAULT_HOST_URL};
use std::future::Future;
#[cfg(not(feature = "async-traits"))]
use std::pin::Pin;
use std::time::Duration;

/// Builder for an http client
#[derive(Debug, Clone)]
pub struct ClientBuilder {
    pub(super) app_version: String,
    pub(super) base_url: String,
    pub(super) request_timeout: Option<Duration>,
    pub(super) connect_timeout: Option<Duration>,
    pub(super) user_agent: String,
    pub(super) proxy_url: Option<Proxy>,
    pub(super) debug: bool,
    pub(super) allow_http: bool,
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ClientBuilder {
    pub fn new() -> Self {
        Self {
            app_version: DEFAULT_APP_VERSION.to_string(),
            user_agent: "NoClient/0.1.0".to_string(),
            base_url: DEFAULT_HOST_URL.to_string(),
            request_timeout: None,
            connect_timeout: None,
            proxy_url: None,
            debug: false,
            allow_http: false,
        }
    }

    /// Set the app version for this client e.g.: my-client@1.4.0+beta.
    /// Note: The default app version is not guaranteed to be accepted by the proton servers.
    pub fn app_version(mut self, version: &str) -> Self {
        self.app_version = version.to_string();
        self
    }

    /// Set the user agent to be submitted with every request.
    pub fn user_agent(mut self, agent: &str) -> Self {
        self.user_agent = agent.to_string();
        self
    }

    /// Set server's base url. By default the proton API server url is used.
    pub fn base_url(mut self, url: &str) -> Self {
        self.base_url = url.to_string();
        self
    }

    /// Set the full request timeout. By default there is no timeout.
    pub fn request_timeout(mut self, duration: Duration) -> Self {
        self.request_timeout = Some(duration);
        self
    }

    /// Set the connection timeout. By default there is no timeout.
    pub fn connect_timeout(mut self, duration: Duration) -> Self {
        self.connect_timeout = Some(duration);
        self
    }

    /// Specify proxy URL for the builder.
    pub fn with_proxy(mut self, proxy: Proxy) -> Self {
        self.proxy_url = Some(proxy);
        self
    }

    /// Allow http request
    pub fn allow_http(mut self) -> Self {
        self.allow_http = true;
        self
    }

    /// Enable request debugging.
    pub fn debug(mut self) -> Self {
        self.debug = true;
        self
    }

    pub fn build<T: TryFrom<ClientBuilder, Error = anyhow::Error> + Clone>(
        self,
    ) -> std::result::Result<T, anyhow::Error> {
        T::try_from(self)
    }
}
pub trait ClientRequest: Sized + Send {
    fn header(self, key: impl AsRef<str>, value: impl AsRef<str>) -> Self;

    fn bearer_token(self, token: impl AsRef<str>) -> Self {
        self.header("authorization", format!("Bearer {}", token.as_ref()))
    }
}

pub trait ClientRequestBuilder: Clone {
    type Request: ClientRequest;
    fn new_request(&self, data: &RequestData) -> Self::Request;
}

/// HTTP Client abstraction Sync.
pub trait ClientSync: ClientRequestBuilder + TryFrom<ClientBuilder, Error = anyhow::Error> {
    fn execute<R: FromResponse>(&self, request: Self::Request) -> Result<R::Output>;
}

/// HTTP Client abstraction Async.
pub trait ClientAsync:
    ClientRequestBuilder + TryFrom<ClientBuilder, Error = anyhow::Error> + Send + Sync
{
    #[cfg(not(feature = "async-traits"))]
    fn execute_async<R: FromResponse>(
        &self,
        request: Self::Request,
    ) -> Pin<Box<dyn Future<Output = Result<R::Output>> + '_>>;

    #[cfg(feature = "async-traits")]
    fn execute_async<R: FromResponse>(
        &self,
        request: Self::Request,
    ) -> impl Future<Output = Result<R::Output>>;
}

pub trait ResponseBodySync {
    type Body: AsRef<[u8]>;
    fn get_body(self) -> Result<Self::Body>;
}

pub trait ResponseBodyAsync {
    type Body: AsRef<[u8]>;

    #[cfg(not(feature = "async-traits"))]
    fn get_body_async(self) -> Pin<Box<dyn Future<Output = Result<Self::Body>>>>;

    #[cfg(feature = "async-traits")]
    fn get_body_async(self) -> impl Future<Output = Result<Self::Body>>;
}

pub trait FromResponse {
    type Output;
    fn from_response_sync<T: ResponseBodySync>(response: T) -> Result<Self::Output>;

    #[cfg(not(feature = "async-traits"))]
    fn from_response_async<T: ResponseBodyAsync + 'static>(
        response: T,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Output>>>>;

    #[cfg(feature = "async-traits")]
    fn from_response_async<T: ResponseBodyAsync + 'static>(
        response: T,
    ) -> impl Future<Output = Result<Self::Output>>;
}
