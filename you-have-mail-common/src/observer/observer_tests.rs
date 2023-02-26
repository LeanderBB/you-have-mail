use crate::backend::null::{new_backend, NullTestAccount};
use crate::backend::Backend;
use crate::MockNotifier;
use crate::{Account, Notifier, ObserverBuilder};
use proton_api_rs::tokio;
use std::sync::Arc;
use std::time::Duration;

async fn new_backend_and_account() -> (Arc<dyn Backend>, Account) {
    let accounts = NullTestAccount {
        email: "foo".to_string(),
        password: "bar".to_string(),
        totp: None,
    };
    let backend = new_backend(&[accounts]);
    let mut account = Account::new(backend.clone(), "foo");
    account.login("bar").await.unwrap();

    assert!(account.is_logged_in());
    (backend, account)
}

#[tokio::test]
async fn notifier_called() {
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

#[tokio::test]
async fn paused_not_call_notifier() {
    let (_, account) = new_backend_and_account().await;

    let mut notifier = MockNotifier::new();
    notifier
        .expect_notify()
        .withf(|account: &Account, num: &usize| account.email() == "foo" && *num == 1)
        .times(0)
        .return_const(());

    notifier.expect_notify_error().times(0);

    let notifier: Box<dyn Notifier> = Box::new(notifier);

    let h = {
        let (observer, task) = ObserverBuilder::new(notifier)
            .poll_interval(Duration::from_millis(10))
            .build();
        let h = tokio::spawn(task);
        observer.pause().await.unwrap();
        observer.add_account(account).await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        observer.shutdown_worker().await.unwrap();
        h
    };

    h.await.unwrap();
}

#[tokio::test]
async fn resume_after_pause_calls_notifier() {
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
            .poll_interval(Duration::from_millis(10))
            .build();
        let h = tokio::spawn(task);
        observer.pause().await.unwrap();
        observer.add_account(account).await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        observer.resume().await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        observer.shutdown_worker().await.unwrap();
        h
    };

    h.await.unwrap();
}

#[tokio::test]
async fn adding_account_with_same_email_twice_is_error() {
    let (_, account) = new_backend_and_account().await;
    let (_, account2) = new_backend_and_account().await;

    let mut notifier = MockNotifier::new();
    notifier
        .expect_notify()
        .withf(|account: &Account, num: &usize| account.email() == "foo" && *num == 1)
        .times(..)
        .return_const(());

    notifier.expect_notify_error().times(0);

    let notifier: Box<dyn Notifier> = Box::new(notifier);

    let h = {
        let (observer, task) = ObserverBuilder::new(notifier)
            .poll_interval(Duration::from_millis(500))
            .build();
        let h = tokio::spawn(task);
        observer.add_account(account).await.unwrap();
        observer.add_account(account2).await.unwrap_err();
        tokio::time::sleep(Duration::from_secs(1)).await;
        observer.shutdown_worker().await.unwrap();
        h
    };

    h.await.unwrap();
}
