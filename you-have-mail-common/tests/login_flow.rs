use proton_api_rs::tokio;
use you_have_mail_common::backend::null::NullTestAccount;
use you_have_mail_common::Account;

#[tokio::test]
async fn test_login_flow() {
    let accounts = NullTestAccount {
        email: "foo".to_string(),
        password: "bar".to_string(),
        totp: "1234".to_string(),
    };
    let backend = you_have_mail_common::backend::null::new_null_backend(&[accounts]);

    Account::login(backend.as_ref(), "z", "b")
        .await
        .expect_err("Account should not be logged in");

    let mut account = Account::login(backend.as_ref(), "foo", "bar")
        .await
        .unwrap();
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
