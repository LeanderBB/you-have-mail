use crate::domain::label::{Label, Type};
use http::{Method, RequestBuilder};
use serde::Deserialize;

#[derive(Copy, Clone)]
pub struct GetLabelsRequest {
    label_type: Type,
}

#[doc(hidden)]
#[derive(Deserialize)]
pub struct GetLabelsResponse {
    #[serde(rename = "Labels")]
    pub labels: Vec<Label>,
}

impl GetLabelsRequest {
    #[must_use]
    pub fn new(label_type: Type) -> Self {
        Self { label_type }
    }
}

impl http::Request for GetLabelsRequest {
    type Response = http::JsonResponse<GetLabelsResponse>;
    const METHOD: Method = Method::Get;

    fn url(&self) -> String {
        "core/v4/labels".to_owned()
    }

    fn build(&self, builder: RequestBuilder) -> http::Result<RequestBuilder> {
        Ok(builder.query("Type", (self.label_type as u8).to_string()))
    }
}
