use proton_api_rs::tokio;
use you_have_mail_common::backend::null::NullTestAccount;
use you_have_mail_common::Account;

#[tokio::test]
async fn test_login_flow() {
    let accounts = NullTestAccount {
        email: "foo".to_string(),
        password: "bar".to_string(),
        totp: Some("1234".to_string()),
        wait_time: None,
    };
    let backend = you_have_mail_common::backend::null::new_backend(&[accounts]);

    let mut account = Account::new(backend, "foo");
    account
        .login("b")
        .await
        .expect_err("Account should not be logged in");

    account.login("bar").await.unwrap();
    assert!(!account.is_logged_out());
    assert!(!account.is_logged_in());
    assert!(account.is_awaiting_totp());

    account.submit_totp("foo").await.expect_err("Expected err");

    account.submit_totp("1234").await.unwrap();

    assert!(!account.is_logged_out());
    assert!(account.is_logged_in());
    assert!(!account.is_awaiting_totp());

    account.logout().await.unwrap();

    assert!(account.is_logged_out());
    assert!(!account.is_logged_in());
    assert!(!account.is_awaiting_totp());
}
