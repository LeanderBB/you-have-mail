use crate::domain::{Label, LabelType};
use crate::http;
use crate::http::RequestData;
use serde::Deserialize;

pub struct GetLabelsRequest {
    label_type: LabelType,
}

#[doc(hidden)]
#[derive(Deserialize)]
pub struct GetLabelsResponse {
    #[serde(rename = "Labels")]
    pub labels: Vec<Label>,
}

impl GetLabelsRequest {
    pub fn new(label_type: LabelType) -> Self {
        Self { label_type }
    }
}

impl http::RequestDesc for GetLabelsRequest {
    type Output = GetLabelsResponse;
    type Response = http::JsonResponse<Self::Output>;

    fn build(&self) -> RequestData {
        RequestData::new(
            http::Method::Get,
            format!("core/v4/labels?Type={}", self.label_type as u8),
        )
    }
}
