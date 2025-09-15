mod utils;
use crate::utils::{new_mock_session_and_server, perform_login};
use proton_api::domain::label::{Id, Label, Type as LabelType, Type};
use proton_api::mocks::{DEFAULT_USER_EMAIL, DEFAULT_USER_PASSWORD};
use proton_api::requests::GetLabelsRequest;

#[test]
fn session_label_fetch() {
    let (client, mut server) = new_mock_session_and_server();
    let _mocks = proton_api::mocks::auth::login_flow(&mut server, false);
    let (_, session) = perform_login(client, DEFAULT_USER_EMAIL, DEFAULT_USER_PASSWORD, false);
    let id = Id("my_label".to_owned());
    let labels = vec![Label {
        id,
        parent_id: None,
        name: "Bar".to_string(),
        path: "Path".to_string(),
        color: "Foo".to_string(),
        label_type: Type::Folder,
        notify: Default::default(),
        display: Default::default(),
        sticky: Default::default(),
        expanded: Default::default(),
        order: 10,
    }];

    let _mock_labels = proton_api::mocks::labels::get_labels(&mut server, Type::Folder, &labels);
    let remote_labels = session
        .execute_with_auth(GetLabelsRequest::new(LabelType::Folder))
        .unwrap()
        .labels;
    assert_eq!(labels, remote_labels);
}
