mod utils;

use crate::utils::{
    new_mock_session_and_server, perform_login, DEFAULT_USER_EMAIL, DEFAULT_USER_PASSWORD,
};
use mockito::{Mock, Server};
use proton_api::domain::event;
use proton_api::mocks::auth::MatchExtension;
use proton_api::requests::{GetLatestEventRequest, GetLatestEventResponse};
use secrecy::ExposeSecret;

#[test]
fn session_login() {
    let (client, mut server) = new_mock_session_and_server();
    let _mocks = proton_api::mocks::auth::login_flow(&mut server, false);
    let _mock = proton_api::mocks::auth::logout(&mut server);
    let (user, session) = perform_login(client, DEFAULT_USER_EMAIL, DEFAULT_USER_PASSWORD, false);
    assert_eq!(user.id.as_ref(), proton_api::mocks::user_id());

    session.logout().unwrap();
}

#[test]
fn session_login_tfa() {
    let (client, mut server) = new_mock_session_and_server();
    let _mocks = proton_api::mocks::auth::login_flow(&mut server, true);
    let (user, _) = perform_login(client, DEFAULT_USER_EMAIL, DEFAULT_USER_PASSWORD, true);
    assert_eq!(user.id.as_ref(), proton_api::mocks::user_id());
}

#[test]
fn session_auto_refresh() {
    let (client, mut server) = new_mock_session_and_server();
    let _mocks = proton_api::mocks::auth::login_flow(&mut server, false);
    let (_, session) = perform_login(client, DEFAULT_USER_EMAIL, DEFAULT_USER_PASSWORD, false);

    let event_id = proton_api::domain::event::Id("foo".to_owned());
    let starting_auth = session.auth_store().read().get().unwrap().cloned().unwrap();

    let _event_failed_mock = mock_get_latest_event_id_401(&mut server);
    let _event_success_mock = mock_get_latest_event_id_refreshed(&mut server, event_id.clone());
    let _auth_refresh_mock = proton_api::mocks::auth::auth_refresh(&mut server);

    let remote_event_id = session
        .execute_with_auth(GetLatestEventRequest {})
        .expect("Failed to get user")
        .event_id;
    assert_eq!(remote_event_id, event_id);

    let refreshed_auth = session.auth_store().read().get().unwrap().cloned().unwrap();

    assert_eq!(starting_auth.uid, refreshed_auth.uid,);

    assert_ne!(
        starting_auth.auth_token.0.expose_secret(),
        refreshed_auth.auth_token.0.expose_secret(),
    );

    assert_ne!(
        starting_auth.refresh_token.0.expose_secret(),
        refreshed_auth.refresh_token.0.expose_secret(),
    );

    assert_eq!(
        refreshed_auth.refresh_token.0.expose_secret(),
        proton_api::mocks::auth::POST_REFRESH_REFRESH_TOKEN
    );
    assert_eq!(
        refreshed_auth.auth_token.0.expose_secret(),
        proton_api::mocks::auth::POST_REFRESH_ACCESS_TOKEN
    );
}
fn mock_get_latest_event_id_401(server: &mut Server) -> Mock {
    server
        .mock("GET", "/core/v4/events/latest")
        .match_auth()
        .with_status(401)
        .create()
}

fn mock_get_latest_event_id_refreshed(server: &mut Server, event_id: event::Id) -> Mock {
    server
        .mock("GET", "/core/v4/events/latest")
        .match_auth_refreshed()
        .with_status(200)
        .with_header("Content-Type", "application/json")
        .with_body(serde_json::to_vec(&GetLatestEventResponse { event_id }).unwrap())
        .create()
}
