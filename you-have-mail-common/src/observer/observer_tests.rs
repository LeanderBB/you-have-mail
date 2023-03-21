use crate::backend::null::{new_backend, NullTestAccount};
use crate::backend::Backend;
use crate::{Account, Notification, Notifier, NullNotifier, ObserverBuilder, Proxy, ProxyProtocol};
use crate::{MockNotifier, Observer};
use mockall::Sequence;
use proton_api_rs::tokio;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

async fn new_backend_and_account() -> (Arc<dyn Backend>, Account) {
    let accounts = NullTestAccount {
        email: "foo".to_string(),
        password: "bar".to_string(),
        totp: None,
        wait_time: None,
    };
    let backend = new_backend(&[accounts]);
    let mut account = Account::new(backend.clone(), "foo", None);
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
        .withf(|n| matches!(n, Notification::AccountAdded(_)))
        .times(1)
        .return_const(());
    notifier
        .expect_notify()
        .withf(|n| {
            matches!(
                n,
                Notification::NewEmail {
                    account: "foo",
                    count: 1,
                    ..
                }
            )
        })
        .times(1..)
        .return_const(());

    let notifier: Box<dyn Notifier> = Box::new(notifier);

    with_observer(
        Duration::from_millis(50),
        notifier,
        move |observer| async move {
            observer.add_account(account).await.unwrap();
            tokio::time::sleep(Duration::from_millis(100)).await;
            observer.shutdown_worker().await.unwrap();
        },
    )
    .await;
}

#[tokio::test]
async fn paused_does_not_call_notifier() {
    let (_, account) = new_backend_and_account().await;

    let mut notifier = MockNotifier::new();
    notifier
        .expect_notify()
        .withf(|n| matches!(n, Notification::AccountAdded(_)))
        .times(1)
        .return_const(());
    notifier
        .expect_notify()
        .withf(|n| {
            matches!(
                n,
                Notification::NewEmail {
                    account: "foo",
                    count: 1,
                    ..
                }
            )
        })
        .times(0)
        .return_const(());

    let notifier: Box<dyn Notifier> = Box::new(notifier);

    with_observer(
        Duration::from_millis(10),
        notifier,
        move |observer| async move {
            observer.pause().await.unwrap();
            observer.add_account(account).await.unwrap();
            tokio::time::sleep(Duration::from_millis(100)).await;
        },
    )
    .await;
}

#[tokio::test]
async fn resume_after_pause_calls_notifier() {
    let (_, account) = new_backend_and_account().await;

    let mut notifier = MockNotifier::new();
    notifier
        .expect_notify()
        .withf(|n| matches!(n, Notification::AccountAdded(_)))
        .times(1)
        .return_const(());
    notifier
        .expect_notify()
        .withf(|n| {
            matches!(
                n,
                Notification::NewEmail {
                    account: "foo",
                    count: 1,
                    ..
                }
            )
        })
        .times(2..)
        .return_const(());

    let notifier: Box<dyn Notifier> = Box::new(notifier);

    with_observer(
        Duration::from_millis(10),
        notifier,
        move |observer| async move {
            observer.pause().await.unwrap();
            observer.add_account(account).await.unwrap();
            tokio::time::sleep(Duration::from_millis(100)).await;
            observer.resume().await.unwrap();
            tokio::time::sleep(Duration::from_millis(400)).await;
        },
    )
    .await;
}

#[tokio::test]
async fn adding_account_with_same_email_twice_is_error() {
    let (_, account) = new_backend_and_account().await;
    let (_, account2) = new_backend_and_account().await;

    let mut notifier = MockNotifier::new();
    notifier
        .expect_notify()
        .withf(|n| matches!(n, Notification::AccountAdded(_)))
        .times(1)
        .return_const(());
    notifier
        .expect_notify()
        .withf(|n| {
            matches!(
                n,
                Notification::NewEmail {
                    account: "foo",
                    count: 1,
                    ..
                }
            )
        })
        .times(..)
        .return_const(());

    let notifier: Box<dyn Notifier> = Box::new(notifier);

    with_observer(
        Duration::from_millis(10),
        notifier,
        move |observer| async move {
            observer.add_account(account).await.unwrap();
            observer.add_account(account2).await.unwrap_err();
        },
    )
    .await;
}

#[tokio::test]
async fn adding_account_after_logout_works() {
    let (_, account) = new_backend_and_account().await;
    let (_, account2) = new_backend_and_account().await;

    let mut notifier = MockNotifier::new();
    let mut sequence = Sequence::new();
    notifier
        .expect_notify()
        .withf(|n| matches!(n, Notification::AccountAdded(_)))
        .times(1)
        .in_sequence(&mut sequence)
        .return_const(());
    notifier
        .expect_notify()
        .withf(|n| matches!(n, Notification::AccountLoggedOut(_)))
        .times(1)
        .in_sequence(&mut sequence)
        .return_const(());
    notifier
        .expect_notify()
        .withf(|n| matches!(n, Notification::AccountOnline(_)))
        .times(1)
        .in_sequence(&mut sequence)
        .return_const(());
    notifier
        .expect_notify()
        .withf(|n| {
            matches!(
                n,
                Notification::NewEmail {
                    account: "foo",
                    count: 1,
                    ..
                }
            )
        })
        .times(1..)
        .return_const(());

    let notifier: Box<dyn Notifier> = Box::new(notifier);

    with_observer(
        Duration::from_millis(10),
        notifier,
        move |observer| async move {
            observer.add_account(account).await.unwrap();
            observer.logout_account("foo").await.unwrap();
            observer.add_account(account2).await.unwrap();
            tokio::time::sleep(Duration::from_secs(1)).await;
        },
    )
    .await;
}

#[tokio::test]
async fn removing_account_produces_remove_notification() {
    let (_, account) = new_backend_and_account().await;

    let mut notifier = MockNotifier::new();
    let mut sequence = Sequence::new();
    notifier
        .expect_notify()
        .withf(|n| matches!(n, Notification::AccountAdded(_)))
        .times(1)
        .in_sequence(&mut sequence)
        .return_const(());
    notifier
        .expect_notify()
        .withf(|n| matches!(n, Notification::AccountRemoved(_)))
        .times(1)
        .in_sequence(&mut sequence)
        .return_const(());

    let notifier: Box<dyn Notifier> = Box::new(notifier);

    with_observer(
        Duration::from_millis(10),
        notifier,
        move |observer| async move {
            observer.add_account(account).await.unwrap();
            observer.remove_account("foo").await.unwrap();
        },
    )
    .await;
}

#[tokio::test]
async fn test_get_set_poll_interval() {
    let notifier: Box<dyn Notifier> = Box::new(NullNotifier {});
    let start_poll_interval = Duration::from_millis(10);

    with_observer(start_poll_interval, notifier, move |observer| async move {
        {
            let current_interval = observer.get_poll_interval().await.unwrap();
            assert_eq!(current_interval, start_poll_interval);
        }
        {
            let new_poll_interval = Duration::from_secs(20);
            observer.set_poll_interval(new_poll_interval).await.unwrap();
            let current_interval = observer.get_poll_interval().await.unwrap();
            assert_eq!(current_interval, new_poll_interval);
        }
    })
    .await;
}

#[tokio::test]
async fn test_proxy_settings() {
    // Apply the same proxy twice in a row and then remove it.
    let (_, account) = new_backend_and_account().await;

    let mut notifier = MockNotifier::new();
    let mut sequence = Sequence::new();
    notifier
        .expect_notify()
        .withf(|n| matches!(n, Notification::AccountAdded(_)))
        .times(1)
        .in_sequence(&mut sequence)
        .return_const(());
    notifier
        .expect_notify()
        .withf(|n| matches!(n, Notification::ProxyApplied(_, Some(_))))
        .times(1)
        .in_sequence(&mut sequence)
        .return_const(());
    notifier
        .expect_notify()
        .withf(|n| matches!(n, Notification::ProxyApplied(_, None)))
        .times(1)
        .in_sequence(&mut sequence)
        .return_const(());
    notifier
        .expect_notify()
        .withf(|n| {
            matches!(
                n,
                Notification::NewEmail {
                    account: "foo",
                    count: 1,
                    ..
                }
            )
        })
        .times(1..)
        .return_const(());

    let notifier: Box<dyn Notifier> = Box::new(notifier);

    with_observer(
        Duration::from_millis(10),
        notifier,
        move |observer| async move {
            let proxy = Proxy {
                protocol: ProxyProtocol::Https,
                auth: None,
                url: "127.0.0.1".into(),
                port: 1080,
            };
            observer.add_account(account).await.unwrap();
            observer
                .set_proxy_settings("foo".to_string(), Some(proxy.clone()))
                .await
                .unwrap();
            observer
                .set_proxy_settings("foo".to_string(), Some(proxy.clone()))
                .await
                .unwrap();
            observer
                .set_proxy_settings("foo".to_string(), None)
                .await
                .unwrap();
            tokio::time::sleep(Duration::from_secs(1)).await;
        },
    )
    .await;
}

async fn with_observer<F, T>(poll_interval: Duration, notifier: Box<dyn Notifier>, f: F)
where
    F: FnOnce(Observer) -> T,
    T: Future<Output = ()>,
{
    let h = {
        let (observer, task) = ObserverBuilder::new(notifier)
            .poll_interval(poll_interval)
            .build();
        let h = tokio::spawn(task);

        (f)(observer.clone()).await;

        observer.shutdown_worker().await.unwrap();
        h
    };
    h.await.unwrap();
}
