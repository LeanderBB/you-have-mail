use crate::domain::label;
use crate::mocks::auth::MatchExtension;
use crate::requests::{GetLabelsRequest, GetLabelsResponse};
use http::Request;
use mockito::{Mock, Server};

pub fn get_labels(server: &mut Server, label_type: label::Type, labels: &[label::Label]) -> Mock {
    let url = GetLabelsRequest::new(label_type).url();
    let url = format!("/{url}?Type={}", label_type as u8);
    let response = GetLabelsResponse {
        labels: labels.to_owned(),
    };
    server
        .mock("GET", url.as_str())
        .match_auth()
        .with_status(200)
        .with_header("Content-Type", "application/json")
        .with_body(serde_json::to_vec(&response).unwrap())
        .create()
}
