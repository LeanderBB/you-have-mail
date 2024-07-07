mod utils;
use crate::utils::{
    create_session_and_server, login, new_session, DEFAULT_USER_EMAIL, DEFAULT_USER_PASSWORD,
};
use proton_api::login::{Error, LoginSequence};
use secrecy::ExposeSecret;

#[test]
fn session_login() {
    let (client, server) = create_session_and_server();

    let (user_id, _) = server
        .create_user(DEFAULT_USER_EMAIL, DEFAULT_USER_PASSWORD)
        .expect("failed to create default user");
    let session = login(client, DEFAULT_USER_EMAIL, DEFAULT_USER_PASSWORD);
    let user = session.user_info().unwrap();
    assert_eq!(user.id.as_ref(), user_id.as_ref());

    session.logout().unwrap();
}

#[test]
fn session_login_auto_refresh() {
    let (client, server) = create_session_and_server();

    let (user_id, _) = server
        .create_user(DEFAULT_USER_EMAIL, DEFAULT_USER_PASSWORD)
        .expect("failed to create default user");

    let session = login(client, DEFAULT_USER_EMAIL, DEFAULT_USER_PASSWORD);

    let starting_auth = session.auth_store().read().get().unwrap().cloned().unwrap();

    let user = session.user_info().expect("Failed to get user");
    assert_eq!(user.id.as_ref(), user_id.as_ref());

    // Simulate authentication refresh
    server
        .set_auth_timeout(std::time::Duration::from_secs(1))
        .expect("Failed to set timeout");
    std::thread::sleep(std::time::Duration::from_secs(1));

    let user = session
        .user_info()
        .expect("Failed to get user post refersh");
    assert_eq!(user.id.as_ref(), user_id.as_ref());

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

    session.logout().expect("Failed to log out");
}

#[test]
fn session_login_invalid_user() {
    let (client, _server) = create_session_and_server();
    let session = new_session(client);

    let mut sequence = LoginSequence::new(session);

    let err = sequence
        .login("bar", DEFAULT_USER_PASSWORD, None)
        .expect_err("Should fail");
    assert!(matches!(err, Error::Api(_)));
}
