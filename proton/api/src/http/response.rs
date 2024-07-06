use crate::http::{FromResponse, ResponseBodyAsync, ResponseBodySync, Result};
use serde::de::DeserializeOwned;
#[cfg(not(feature = "async-traits"))]
use std::future::Future;
use std::marker::PhantomData;
#[cfg(not(feature = "async-traits"))]
use std::pin::Pin;

#[derive(Copy, Clone)]
pub struct NoResponse {}

impl FromResponse for NoResponse {
    type Output = ();

    fn from_response_sync<T: ResponseBodySync>(_: T) -> Result<Self::Output> {
        Ok(())
    }

    #[cfg(not(feature = "async-traits"))]
    fn from_response_async<T: ResponseBodyAsync>(
        _: T,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Output>>>> {
        Box::pin(async { Ok(()) })
    }

    #[cfg(feature = "async-traits")]
    async fn from_response_async<T: ResponseBodyAsync>(_: T) -> Result<Self::Output> {
        Ok(())
    }
}

pub struct JsonResponse<T: DeserializeOwned>(PhantomData<T>);

impl<T: DeserializeOwned> FromResponse for JsonResponse<T> {
    type Output = T;

    fn from_response_sync<R: ResponseBodySync>(response: R) -> Result<Self::Output> {
        let body = response.get_body()?;
        let r = serde_json::from_slice(body.as_ref())?;
        Ok(r)
    }

    #[cfg(not(feature = "async-traits"))]
    fn from_response_async<R: ResponseBodyAsync + 'static>(
        response: R,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Output>>>> {
        Box::pin(async move {
            let body = response.get_body_async().await?;
            let r = serde_json::from_slice(body.as_ref())?;
            Ok(r)
        })
    }

    #[cfg(feature = "async-traits")]
    async fn from_response_async<R: ResponseBodyAsync + 'static>(
        response: R,
    ) -> Result<Self::Output> {
        let body = response.get_body_async().await?;
        let r = serde_json::from_slice(body.as_ref())?;
        Ok(r)
    }
}

#[derive(Copy, Clone)]
pub struct StringResponse {}

impl FromResponse for StringResponse {
    type Output = String;

    fn from_response_sync<R: ResponseBodySync>(response: R) -> Result<Self::Output> {
        let body = response.get_body()?;
        Ok(String::from_utf8_lossy(body.as_ref()).to_string())
    }

    #[cfg(not(feature = "async-traits"))]
    fn from_response_async<R: ResponseBodyAsync + 'static>(
        response: R,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Output>>>> {
        Box::pin(async move {
            let body = response.get_body_async().await?;
            Ok(String::from_utf8_lossy(body.as_ref()).to_string())
        })
    }

    #[cfg(feature = "async-traits")]
    async fn from_response_async<R: ResponseBodyAsync + 'static>(
        response: R,
    ) -> Result<Self::Output> {
        let body = response.get_body_async().await?;
        Ok(String::from_utf8_lossy(body.as_ref()).to_string())
    }
}
