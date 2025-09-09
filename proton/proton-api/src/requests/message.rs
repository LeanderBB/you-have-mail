use crate::domain::errors::{APIError, APIErrorDesc, OPERATION_SUCCESS};
use crate::domain::label;
use crate::domain::message::Id;
use serde::{Deserialize, Serialize};
use you_have_mail_http::{Method, RequestBuilder};

/// Response items returned for message operations.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "mocks", derive(serde::Serialize))]
#[serde(rename_all = "PascalCase")]
pub struct OperationResponse {
    #[serde(rename = "ID")]
    pub id: Id,
    pub response: APIErrorDesc,
}

impl OperationResponse {
    /// Create new operation success response
    #[must_use]
    pub fn ok(id: Id) -> Self {
        Self {
            id,
            response: APIErrorDesc {
                code: OPERATION_SUCCESS,
                details: None,
                error: None,
            },
        }
    }

    /// Check whether this operation was successful.
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.response.code == OPERATION_SUCCESS
    }

    /// Convert the current response into a result type.
    ///
    /// # Errors
    ///
    /// Converts into an `APIError` if the operation did not succeed.
    pub fn into_result(self) -> Result<(), APIError> {
        if self.is_success() {
            Ok(())
        } else {
            Err(APIError::with_desc(200, self.response))
        }
    }
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "mocks", derive(serde::Serialize))]
#[serde(rename_all = "PascalCase")]
pub struct PutMarkMessageReadResponse {
    pub responses: Vec<OperationResponse>,
}

/// Mark the given message ids as read.
#[derive(Debug, Serialize)]
pub struct PutMarkMessageReadRequest {
    #[serde(rename = "IDs")]
    pub ids: Vec<Id>,
}
impl PutMarkMessageReadRequest {
    pub fn new(ids: impl IntoIterator<Item = Id>) -> Self {
        Self {
            ids: ids.into_iter().collect(),
        }
    }
}

impl you_have_mail_http::Request for PutMarkMessageReadRequest {
    type Response = you_have_mail_http::JsonResponse<PutMarkMessageReadResponse>;
    const METHOD: Method = Method::Put;

    fn url(&self) -> String {
        "mail/v4/messages/read".to_owned()
    }
    fn build(&self, builder: RequestBuilder) -> you_have_mail_http::Result<RequestBuilder> {
        Ok(builder.json(self))
    }
}

/// Mark the given message ids as read.
#[derive(Debug, Serialize)]
pub struct PutLabelMessageRequest {
    #[serde(rename = "IDs")]
    pub ids: Vec<Id>,
    #[serde(rename = "LabelID")]
    pub label_id: label::Id,
}

impl PutLabelMessageRequest {
    pub fn new(label_id: label::Id, ids: impl IntoIterator<Item = Id>) -> Self {
        Self {
            ids: ids.into_iter().collect(),
            label_id,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[cfg_attr(feature = "mocks", derive(serde::Serialize))]
pub struct PutLabelMessageResponse {
    pub responses: Vec<OperationResponse>,
}
impl you_have_mail_http::Request for PutLabelMessageRequest {
    type Response = you_have_mail_http::JsonResponse<PutLabelMessageResponse>;
    const METHOD: Method = Method::Put;

    fn url(&self) -> String {
        "mail/v4/messages/label".to_owned()
    }

    fn build(&self, builder: RequestBuilder) -> you_have_mail_http::Result<RequestBuilder> {
        Ok(builder.json(self))
    }
}
