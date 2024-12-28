use crate::domain::{label, message};
use crate::mocks::auth::MatchExtension;
use crate::requests::{
    PutLabelMessageRequest, PutLabelMessageResponse, PutMarkMessageReadRequest,
    PutMarkMessageReadResponse,
};
use http::Request;
use mockito::{Mock, Server};

/// Mock marking message as read with the given `ids` returning the given `response`.
pub fn mark_message_read(
    server: &mut Server,
    ids: Vec<message::Id>,
    response: &PutMarkMessageReadResponse,
) -> Mock {
    let request = PutMarkMessageReadRequest::new(ids);
    server
        .mock("PUT", format!("/{}", request.url()).as_str())
        .match_body(serde_json::to_vec(&request).unwrap())
        .match_auth()
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(serde_json::to_vec(&response).unwrap())
        .create()
}

/// Mock labeling the messages with `ids` with `label_id` returning the given `response`.
pub fn label_message(
    server: &mut Server,
    label_id: label::Id,
    ids: Vec<message::Id>,
    response: &PutLabelMessageResponse,
) -> Mock {
    let request = PutLabelMessageRequest::new(label_id, ids);
    server
        .mock("PUT", format!("/{}", request.url()).as_str())
        .match_body(serde_json::to_vec(&request).unwrap())
        .match_auth()
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(serde_json::to_vec(response).unwrap())
        .create()
}
