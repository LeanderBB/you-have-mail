use crate::domain::label::{Label, Type};
use serde::Deserialize;
use you_have_mail_http::{Method, RequestBuilder};

#[derive(Copy, Clone)]
pub struct GetLabelsRequest {
    label_type: Type,
}

#[doc(hidden)]
#[derive(Deserialize)]
#[cfg_attr(feature = "mocks", derive(serde::Serialize))]
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

impl you_have_mail_http::Request for GetLabelsRequest {
    type Response = you_have_mail_http::JsonResponse<GetLabelsResponse>;
    const METHOD: Method = Method::Get;

    fn url(&self) -> String {
        "core/v4/labels".to_owned()
    }

    fn build(&self, builder: RequestBuilder) -> you_have_mail_http::Result<RequestBuilder> {
        Ok(builder.query("Type", (self.label_type as u8).to_string()))
    }
}
