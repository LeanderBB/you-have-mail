use crate::backend::null::{new_null_backend, NullTestAccount};
use crate::backend::Backend;
use crate::MockNotifier;
use crate::{Account, Notifier, ObserverBuilder};
use proton_api_rs::tokio;
use std::time::Duration;

async fn new_backend_and_account() -> (Box<dyn Backend>, Account) {
    let accounts = NullTestAccount {
        email: "foo".to_string(),
        password: "bar".to_string(),
        totp: String::new(),
    };
    let backend = new_null_backend(&[accounts]);
    let account = Account::login(backend.as_ref(), "foo", "bar")
        .await
        .unwrap();

    assert!(account.is_logged_in());
    (backend, account)
}

#[tokio::test]
async fn observer_calls_notifier() {
    let (_, account) = new_backend_and_account().await;

    let mut notifier = MockNotifier::new();
    notifier
        .expect_notify()
        .withf(|account: &Account, num: &usize| account.email() == "foo" && *num == 1)
        .times(1..)
        .return_const(());

    notifier.expect_notify_error().times(0);

    let notifier: Box<dyn Notifier> = Box::new(notifier);

    let h = {
        let (observer, task) = ObserverBuilder::new(notifier)
            .poll_interval(Duration::from_millis(500))
            .build();
        let h = tokio::spawn(task);
        observer.add_account(account).await.unwrap();
        tokio::time::sleep(Duration::from_secs(1)).await;
        observer.shutdown_worker().await.unwrap();
        h
    };

    h.await.unwrap();
}
