use crate::utils::{
    new_mock_session_and_server, perform_login, DEFAULT_USER_EMAIL, DEFAULT_USER_PASSWORD,
};
use proton_api::domain::event;
use proton_api::domain::event::MoreEvents;
use proton_api::requests::GetEventRequest;

mod utils;
#[test]
fn get_events() {
    // Check get events API call.
    let (client, mut server) = new_mock_session_and_server();
    let _mocks = proton_api::mocks::auth::login_flow(&mut server, false);
    let (_, session) = perform_login(client, DEFAULT_USER_EMAIL, DEFAULT_USER_PASSWORD, false);

    let id = event::Id("foo".to_owned());
    let event = event::Event {
        event_id: id.clone(),
        more: MoreEvents::Yes,
        messages: None,
        labels: None,
    };

    let _get_event_mock = proton_api::mocks::events::get_event(&mut server, &event);
    let remote_event = session
        .execute_with_auth(GetEventRequest::new(&id))
        .unwrap();
    assert_eq!(remote_event, event);
}
