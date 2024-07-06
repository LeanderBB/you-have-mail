use crate::utils::{
    create_session_and_server, ClientSync, DEFAULT_USER_EMAIL, DEFAULT_USER_PASSWORD,
};
use proton_api_rs::domain::LabelType;
use proton_api_rs::http::Sequence;
use proton_api_rs::{Session, SessionType};
use secrecy::Secret;

#[test]
fn session_label_fetch() {
    let (client, server) = create_session_and_server::<ClientSync>();

    let (user_id, _) = server
        .create_user(DEFAULT_USER_EMAIL, DEFAULT_USER_PASSWORD)
        .expect("failed to create default user");

    let folder_id = server
        .create_label(&user_id, "my_folder", None, LabelType::Folder as i32)
        .expect("Failed to create folder");

    let label_id = server
        .create_label(&user_id, "my_label", None, LabelType::Label as i32)
        .expect("Failed to create folder");

    let auth_result = Session::login(
        DEFAULT_USER_EMAIL,
        &Secret::<String>::new(DEFAULT_USER_PASSWORD.to_string()),
        None,
    )
    .do_sync(&client)
    .expect("Failed to login");

    assert!(matches!(auth_result, SessionType::Authenticated(_)));

    if let SessionType::Authenticated(s) = auth_result {
        {
            let folders = s
                .get_labels(LabelType::Folder)
                .do_sync(&client)
                .expect("Failed to get folder labels");
            assert_eq!(1, folders.len());
            assert_eq!(folder_id.as_ref(), folders[0].id.0);
            assert_eq!("my_folder", folders[0].name);
        }

        {
            let labels = s
                .get_labels(LabelType::Label)
                .do_sync(&client)
                .expect("Failed to get folder labels");
            assert_eq!(1, labels.len());
            assert_eq!(label_id.as_ref(), labels[0].id.0);
            assert_eq!("my_label", labels[0].name);
        }
    }
}
