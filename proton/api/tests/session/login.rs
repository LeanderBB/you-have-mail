use crate::utils::{
    create_session_and_server, ClientASync, ClientSync, DEFAULT_USER_EMAIL, DEFAULT_USER_PASSWORD,
};
use proton_api_rs::domain::SecretString;
use proton_api_rs::http::Sequence;
use proton_api_rs::{http, LoginError, Session, SessionType};
use secrecy::{ExposeSecret, Secret};
use tokio;

#[test]
fn session_login() {
    let (client, server) = create_session_and_server::<ClientSync>();

    let (user_id, _) = server
        .create_user(DEFAULT_USER_EMAIL, DEFAULT_USER_PASSWORD)
        .expect("failed to create default user");
    let auth_result = Session::login(
        DEFAULT_USER_EMAIL,
        &Secret::<String>::new(DEFAULT_USER_PASSWORD.to_string()),
        None,
    )
    .do_sync(&client)
    .expect("Failed to login");

    assert!(matches!(auth_result, SessionType::Authenticated(_)));

    if let SessionType::Authenticated(s) = auth_result {
        let user = s.get_user().do_sync(&client).expect("Failed to get user");
        assert_eq!(user.id.as_ref(), user_id.as_ref());

        s.logout().do_sync(&client).expect("Failed to logout")
    }
}

#[test]
fn session_login_auto_refresh() {
    let (client, server) = create_session_and_server::<ClientSync>();

    let (user_id, _) = server
        .create_user(DEFAULT_USER_EMAIL, DEFAULT_USER_PASSWORD)
        .expect("failed to create default user");
    let auth_result = Session::login(
        DEFAULT_USER_EMAIL,
        &Secret::<String>::new(DEFAULT_USER_PASSWORD.to_string()),
        None,
    )
    .do_sync(&client)
    .expect("Failed to login");

    assert!(matches!(auth_result, SessionType::Authenticated(_)));

    if let SessionType::Authenticated(s) = auth_result {
        let user = s.get_user().do_sync(&client).expect("Failed to get user");
        assert_eq!(user.id.as_ref(), user_id.as_ref());

        let rs = s.get_refresh_data();
        server
            .set_auth_timeout(std::time::Duration::from_secs(1))
            .expect("Failed to set timeout");
        std::thread::sleep(std::time::Duration::from_secs(1));

        let user = s.get_user().do_sync(&client).expect("Failed to get user");
        assert_eq!(user.id.as_ref(), user_id.as_ref());

        let rs_post_refresh = s.get_refresh_data();

        assert_eq!(
            rs.user_uid.expose_secret(),
            rs_post_refresh.user_uid.expose_secret()
        );

        assert_ne!(
            rs.token.expose_secret(),
            rs_post_refresh.token.expose_secret()
        );

        s.logout().do_sync(&client).expect("Failed to logout")
    }
}

#[tokio::test()]
async fn session_login_async() {
    let (client, server) = create_session_and_server::<ClientASync>();

    let (user_id, _) = server
        .create_user(DEFAULT_USER_EMAIL, DEFAULT_USER_PASSWORD)
        .expect("failed to create default user");
    let auth_result = Session::login(
        DEFAULT_USER_EMAIL,
        &Secret::<String>::new(DEFAULT_USER_PASSWORD.to_string()),
        None,
    )
    .do_async(&client)
    .await
    .expect("Failed to login");

    assert!(matches!(auth_result, SessionType::Authenticated(_)));

    if let SessionType::Authenticated(s) = auth_result {
        let user = s
            .get_user()
            .do_async(&client)
            .await
            .expect("Failed to get user");
        assert_eq!(user.id.as_ref(), user_id.as_ref());

        s.logout()
            .do_async(&client)
            .await
            .expect("Failed to logout")
    }
}

#[test]
fn session_login_invalid_user() {
    let (client, _server) = create_session_and_server::<ClientSync>();
    let auth_result = Session::login(
        "bar",
        &SecretString::new(DEFAULT_USER_PASSWORD.into()),
        None,
    )
    .do_sync(&client);

    assert!(matches!(
        auth_result,
        Err(LoginError::Request(http::Error::API(_)))
    ));
}
