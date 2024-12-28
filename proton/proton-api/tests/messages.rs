mod utils;

use crate::utils::{new_mock_session_and_server, perform_login};
use proton_api::domain::label;
use proton_api::domain::message::Id;
use proton_api::mocks::{DEFAULT_USER_EMAIL, DEFAULT_USER_PASSWORD};
use proton_api::requests::{
    OperationResponse, PutLabelMessageRequest, PutLabelMessageResponse, PutMarkMessageReadRequest,
    PutMarkMessageReadResponse,
};

#[test]
fn mark_message_read() {
    let (client, mut server) = new_mock_session_and_server();
    let _mocks = proton_api::mocks::auth::login_flow(&mut server, false);
    let (_, session) = perform_login(client, DEFAULT_USER_EMAIL, DEFAULT_USER_PASSWORD, false);
    let id = Id("my_message".to_owned());

    let _mock_mark_read = proton_api::mocks::message::mark_message_read(
        &mut server,
        vec![id.clone()],
        &PutMarkMessageReadResponse {
            responses: vec![OperationResponse::ok(id.clone())],
        },
    );
    let response = session
        .execute_with_auth(PutMarkMessageReadRequest::new([id.clone()]))
        .unwrap();
    assert_eq!(response.responses.len(), 1);
    assert_eq!(response.responses[0].id, id);
    assert!(response.responses[0].is_success());
}

#[test]
fn label_message() {
    let (client, mut server) = new_mock_session_and_server();
    let _mocks = proton_api::mocks::auth::login_flow(&mut server, false);
    let (_, session) = perform_login(client, DEFAULT_USER_EMAIL, DEFAULT_USER_PASSWORD, false);
    let id = Id("my_message".to_owned());
    let label_id = label::Id::trash();

    let _mock = proton_api::mocks::message::label_message(
        &mut server,
        label_id.clone(),
        vec![id.clone()],
        &PutLabelMessageResponse {
            responses: vec![OperationResponse::ok(id.clone())],
        },
    );
    let response = session
        .execute_with_auth(PutLabelMessageRequest::new(label_id, [id.clone()]))
        .unwrap();
    assert_eq!(response.responses.len(), 1);
    assert_eq!(response.responses[0].id, id);
    assert!(response.responses[0].is_success());
}
