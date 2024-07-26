use crate::domain::event;
use crate::mocks::auth::MatchExtension;
use crate::requests::{GetEventRequest, GetLatestEventResponse};
use http::Request;
use mockito::{Mock, Server};

/// Mock get latest event request.
pub fn get_latest_event_id(server: &mut Server, event_id: event::Id) -> Mock {
    server
        .mock("GET", "/core/v4/events/latest")
        .match_auth()
        .with_status(200)
        .with_header("Content-Type", "application/json")
        .with_body(serde_json::to_vec(&GetLatestEventResponse { event_id }).unwrap())
        .create()
}

/// Mock get event by `event_id`.
pub fn get_event(server: &mut Server, event_id: &event::Id, event: &event::Event) -> Mock {
    let url = GetEventRequest::new(event_id).url();
    let url = format!("/{url}");
    server
        .mock("GET", url.as_str())
        .match_auth()
        .with_status(200)
        .with_body(serde_json::to_vec(event).unwrap())
        .create()
}
