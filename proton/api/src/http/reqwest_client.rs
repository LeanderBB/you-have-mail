use crate::http::{
    ClientAsync, ClientBuilder, ClientRequest, ClientRequestBuilder, Error, FromResponse, Method,
    RequestData, ResponseBodyAsync, X_PM_APP_VERSION_HEADER,
};
use crate::requests::APIError;
use bytes::Bytes;
use reqwest;

#[cfg(not(feature = "async-traits"))]
use std::future::Future;
#[cfg(not(feature = "async-traits"))]
use std::pin::Pin;

#[derive(Debug, Clone)]
pub struct ReqwestClient {
    client: reqwest::Client,
    base_url: String,
}

impl TryFrom<ClientBuilder> for ReqwestClient {
    type Error = anyhow::Error;

    fn try_from(value: ClientBuilder) -> Result<Self, Self::Error> {
        use reqwest::tls::Version;
        let mut header_map = reqwest::header::HeaderMap::new();
        header_map.insert(
            X_PM_APP_VERSION_HEADER,
            reqwest::header::HeaderValue::from_str(&value.app_version)
                .map_err(|e| anyhow::anyhow!(e))?,
        );

        let mut builder = reqwest::ClientBuilder::new();

        if let Some(proxy) = value.proxy_url {
            let proxy = reqwest::Proxy::all(proxy.as_url())?;
            builder = builder.proxy(proxy);
        }

        if let Some(d) = value.connect_timeout {
            builder = builder.connect_timeout(d)
        }

        if let Some(d) = value.request_timeout {
            builder = builder.timeout(d)
        }

        builder = builder
            .min_tls_version(Version::TLS_1_2)
            .https_only(!value.allow_http)
            .cookie_store(true)
            .user_agent(value.user_agent)
            .default_headers(header_map);

        Ok(Self {
            client: builder.build()?,
            base_url: value.base_url,
        })
    }
}

impl From<reqwest::Error> for Error {
    fn from(value: reqwest::Error) -> Self {
        // Check timeout before all other errors as it can be produced by multiple
        // reqwest error kinds.
        if value.is_timeout() {
            return Error::Timeout(anyhow::Error::new(value));
        }

        if value.is_connect() {
            return Error::Connection(anyhow::Error::new(value));
        }

        if value.is_body() {
            Error::Request(anyhow::Error::new(value))
        } else if value.is_redirect() {
            Error::Redirect(
                value
                    .url()
                    .map(|v| v.to_string())
                    .unwrap_or("Unknown URL".to_string()),
                anyhow::Error::new(value),
            )
        } else if value.is_request() {
            Error::Request(anyhow::Error::new(value))
        } else {
            Error::Other(anyhow::Error::new(value))
        }
    }
}

struct ReqwestResponse(reqwest::Response);

pub struct ReqwestRequest(reqwest::RequestBuilder);

impl ClientRequest for ReqwestRequest {
    fn header(self, key: impl AsRef<str>, value: impl AsRef<str>) -> Self {
        Self(self.0.header(key.as_ref(), value.as_ref()))
    }
}

impl ResponseBodyAsync for ReqwestResponse {
    type Body = Bytes;

    #[cfg(not(feature = "async-traits"))]
    fn get_body_async(self) -> Pin<Box<dyn Future<Output = crate::http::Result<Self::Body>>>> {
        Box::pin(async {
            let bytes = self.0.bytes().await?;
            Ok(bytes)
        })
    }

    #[cfg(feature = "async-traits")]
    async fn get_body_async(self) -> crate::http::Result<Self::Body> {
        let bytes = self.0.bytes().await?;
        Ok(bytes)
    }
}

impl ClientRequestBuilder for ReqwestClient {
    type Request = ReqwestRequest;

    fn new_request(&self, data: &RequestData) -> Self::Request {
        let final_url = format!("{}/{}", self.base_url, data.url);

        let mut request = match data.method {
            Method::Delete => self.client.delete(&final_url),
            Method::Get => self.client.get(&final_url),
            Method::Put => self.client.put(&final_url),
            Method::Post => self.client.post(&final_url),
            Method::Patch => self.client.patch(&final_url),
        };

        // Set headers.
        for (header, value) in &data.headers {
            request = request.header(header, value);
        }

        if let Some(body) = &data.body {
            request = request.body(body.clone())
        }

        ReqwestRequest(request)
    }
}

impl ReqwestClient {
    pub async fn direct_exec<R: FromResponse>(
        &self,
        r: ReqwestRequest,
    ) -> crate::http::Result<R::Output> {
        let response = r.0.send().await?;

        let status = response.status().as_u16();

        if status >= 400 {
            let body = response
                .bytes()
                .await
                .map_err(|_| Error::API(APIError::new(status)))?;

            return Err(Error::API(APIError::with_status_and_body(
                status,
                body.as_ref(),
            )));
        }

        R::from_response_async(ReqwestResponse(response)).await
    }
}

impl ClientAsync for ReqwestClient {
    #[cfg(not(feature = "async-traits"))]
    fn execute_async<R: FromResponse>(
        &self,
        r: Self::Request,
    ) -> Pin<Box<dyn Future<Output = crate::http::Result<R::Output>> + '_>> {
        Box::pin(async move { self.direct_exec::<R>(r).await })
    }

    #[cfg(feature = "async-traits")]
    async fn execute_async<R: FromResponse>(
        &self,
        request: Self::Request,
    ) -> crate::http::Result<R::Output> {
        self.direct_exec::<R>(request).await
    }
}
