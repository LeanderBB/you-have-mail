//! UReq HTTP client implementation.

use crate::http::X_PM_APP_VERSION_HEADER;
use crate::http::{
    ClientBuilder, ClientRequest, ClientRequestBuilder, ClientSync, Error, FromResponse, Method,
    RequestData, ResponseBodySync,
};
use crate::requests::APIError;
use log::debug;
use std::io;
use std::io::Read;
use ureq;

#[derive(Debug, Clone)]
pub struct UReqClient {
    agent: ureq::Agent,
    app_version: String,
    base_url: String,
    debug: bool,
}

impl TryFrom<ClientBuilder> for UReqClient {
    type Error = anyhow::Error;

    fn try_from(value: ClientBuilder) -> Result<Self, Self::Error> {
        let mut builder = ureq::AgentBuilder::new();

        if let Some(d) = value.request_timeout {
            builder = builder.timeout(d);
        }

        if let Some(d) = value.connect_timeout {
            builder = builder.timeout_connect(d)
        }

        if let Some(proxy) = value.proxy_url {
            let proxy = ureq::Proxy::new(proxy.as_url())?;
            builder = builder.proxy(proxy);
        }

        if !value.allow_http {
            builder = builder.https_only(true)
        }

        let agent = builder
            .user_agent(&value.user_agent)
            .max_idle_connections(0)
            .max_idle_connections_per_host(0)
            .build();

        Ok(Self {
            agent,
            app_version: value.app_version,
            base_url: value.base_url,
            debug: value.debug,
        })
    }
}

impl From<ureq::Error> for Error {
    fn from(value: ureq::Error) -> Self {
        match value {
            ureq::Error::Status(status, response) => {
                if let Ok(body) = safe_read_body(response) {
                    return Error::API(APIError::with_status_and_body(status, &body));
                }

                Error::API(APIError::new(status))
            }
            ureq::Error::Transport(t) => match t.kind() {
                ureq::ErrorKind::InvalidUrl => Error::Request(t.into()),
                ureq::ErrorKind::UnknownScheme => Error::Request(t.into()),
                ureq::ErrorKind::Dns => Error::Connection(t.into()),
                ureq::ErrorKind::InsecureRequestHttpsOnly => Error::Request(t.into()),
                ureq::ErrorKind::ConnectionFailed => Error::Connection(t.into()),
                ureq::ErrorKind::TooManyRedirects => Error::Redirect(
                    t.url()
                        .map(|u| u.to_string())
                        .unwrap_or("Unknown url".to_string()),
                    t.into(),
                ),
                ureq::ErrorKind::BadStatus => Error::Request(t.into()),
                ureq::ErrorKind::BadHeader => Error::Request(t.into()),
                ureq::ErrorKind::Io => Error::Connection(t.into()),
                ureq::ErrorKind::InvalidProxyUrl => Error::Connection(t.into()),
                ureq::ErrorKind::ProxyConnect => Error::Connection(t.into()),
                ureq::ErrorKind::ProxyUnauthorized => Error::Connection(t.into()),
                ureq::ErrorKind::HTTP => Error::Request(t.into()),
            },
        }
    }
}

struct UReqResponse(ureq::Response);

impl ResponseBodySync for UReqResponse {
    type Body = Vec<u8>;

    fn get_body(self) -> crate::http::Result<Self::Body> {
        let body = safe_read_body(self.0)
            .map_err(|e| Error::Request(anyhow::anyhow!("Failed to read response body {e}")))?;
        Ok(body)
    }
}

struct UReqDebugResponse(ureq::Response);

impl ResponseBodySync for UReqDebugResponse {
    type Body = Vec<u8>;

    fn get_body(self) -> crate::http::Result<Self::Body> {
        let body = safe_read_body(self.0)
            .map_err(|e| Error::Request(anyhow::anyhow!("Failed to read response body {e}")))?;

        let body_str = String::from_utf8_lossy(&body);
        debug!("Request Body: {}", body_str);

        Ok(body)
    }
}

pub struct UReqRequest {
    request: ureq::Request,
    body: Option<bytes::Bytes>,
}

impl ClientRequest for UReqRequest {
    fn header(mut self, key: impl AsRef<str>, value: impl AsRef<str>) -> Self {
        self.request = self.request.set(key.as_ref(), value.as_ref());
        self
    }
}

impl ClientRequestBuilder for UReqClient {
    type Request = UReqRequest;

    fn new_request(&self, request: &RequestData) -> Self::Request {
        let final_url = format!("{}/{}", self.base_url, request.url);
        let mut ureq_request = match request.method {
            Method::Delete => self.agent.delete(&final_url),
            Method::Get => self.agent.get(&final_url),
            Method::Put => self.agent.put(&final_url),
            Method::Post => self.agent.post(&final_url),
            Method::Patch => self.agent.patch(&final_url),
        };

        // Set app version.
        ureq_request = ureq_request.set(X_PM_APP_VERSION_HEADER, &self.app_version);

        // Set headers.
        for (header, value) in &request.headers {
            ureq_request = ureq_request.set(header, value);
        }

        Self::Request {
            request: ureq_request,
            body: request.body.clone(),
        }
    }
}

impl ClientSync for UReqClient {
    fn execute<R: FromResponse>(&self, request: Self::Request) -> Result<R::Output, Error> {
        let ureq_response = if let Some(body) = request.body {
            request.request.send_bytes(body.as_ref())?
        } else {
            request.request.call()?
        };

        if !self.debug {
            R::from_response_sync(UReqResponse(ureq_response))
        } else {
            R::from_response_sync(UReqDebugResponse(ureq_response))
        }
    }
}

fn safe_read_body(response: ureq::Response) -> Result<Vec<u8>, io::Error> {
    let mut vec = vec![];

    if let Some(length) = response.header("Content-Length") {
        if let Ok(len) = length.parse::<usize>() {
            if len == 0 {
                return Ok(vec![]);
            }
            vec.reserve(len);
        }
    }

    let _ = response
        .into_reader()
        .take(10_000_000)
        .read_to_end(&mut vec)?;

    Ok(vec)
}
