use crate::backend::null::{new_backend, NullTestAccount};
use crate::backend::Backend;
use crate::{Account, Notification, Notifier, NullNotifier, ObserverBuilder, Proxy, ProxyProtocol};
use crate::{MockNotifier, Observer};
use mockall::Sequence;
use std::sync::Arc;
use std::time::Duration;

fn new_backend_and_account() -> (Arc<dyn Backend>, Account) {
    let accounts = NullTestAccount {
        email: "foo".to_string(),
        password: "bar".to_string(),
        totp: None,
        wait_time: None,
    };
    let backend = new_backend(&[accounts]);
    let mut account = Account::new(backend.clone(), "foo", None);
    account.login("bar", None).unwrap();

    assert!(account.is_logged_in());
    (backend, account)
}

#[test]
fn notifier_called() {
    let (_, account) = new_backend_and_account();

    let mut notifier = MockNotifier::new();
    notifier
        .expect_notify()
        .withf(|n| matches!(n, Notification::AccountAdded(_)))
        .times(1)
        .return_const(());
    notifier
        .expect_notify()
        .withf(|n| matches!(n, Notification::NewEmail { account: "foo", .. }))
        .times(1..)
        .return_const(());

    let notifier: Box<dyn Notifier> = Box::new(notifier);

    with_observer(Duration::from_millis(50), notifier, move |observer| {
        observer.add_account(account).unwrap();
        std::thread::sleep(Duration::from_millis(100));
        observer.shutdown_worker().unwrap();
    });
}

#[test]
fn paused_does_not_call_notifier() {
    let (_, account) = new_backend_and_account();

    let mut notifier = MockNotifier::new();
    notifier
        .expect_notify()
        .withf(|n| matches!(n, Notification::AccountAdded(_)))
        .times(1)
        .return_const(());
    notifier
        .expect_notify()
        .withf(|n| matches!(n, Notification::NewEmail { account: "foo", .. }))
        .times(0)
        .return_const(());

    let notifier: Box<dyn Notifier> = Box::new(notifier);

    with_observer(Duration::from_millis(10), notifier, move |observer| {
        observer.pause().unwrap();
        observer.add_account(account).unwrap();
        std::thread::sleep(Duration::from_millis(100));
    });
}

#[test]
fn resume_after_pause_calls_notifier() {
    let (_, account) = new_backend_and_account();

    let mut notifier = MockNotifier::new();
    notifier
        .expect_notify()
        .withf(|n| matches!(n, Notification::AccountAdded(_)))
        .times(1)
        .return_const(());
    notifier
        .expect_notify()
        .withf(|n| matches!(n, Notification::NewEmail { account: "foo", .. }))
        .times(2..)
        .return_const(());

    let notifier: Box<dyn Notifier> = Box::new(notifier);

    with_observer(Duration::from_millis(10), notifier, move |observer| {
        observer.pause().unwrap();
        observer.add_account(account).unwrap();
        std::thread::sleep(Duration::from_millis(100));
        observer.resume().unwrap();
        std::thread::sleep(Duration::from_millis(400));
    });
}

#[test]
fn adding_account_with_same_email_twice_is_error() {
    let (_, account) = new_backend_and_account();
    let (_, account2) = new_backend_and_account();

    let mut notifier = MockNotifier::new();
    notifier
        .expect_notify()
        .withf(|n| matches!(n, Notification::AccountAdded(_)))
        .times(1)
        .return_const(());
    notifier
        .expect_notify()
        .withf(|n| matches!(n, Notification::NewEmail { account: "foo", .. }))
        .times(..)
        .return_const(());

    let notifier: Box<dyn Notifier> = Box::new(notifier);

    with_observer(Duration::from_millis(10), notifier, move |observer| {
        observer.add_account(account).unwrap();
        observer.add_account(account2).unwrap_err();
    });
}

#[test]
fn adding_account_after_logout_works() {
    let (_, account) = new_backend_and_account();
    let (_, account2) = new_backend_and_account();

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
        .withf(|n| matches!(n, Notification::AccountOnline(_)))
        .times(1)
        .in_sequence(&mut sequence)
        .return_const(());
    notifier
        .expect_notify()
        .withf(|n| matches!(n, Notification::NewEmail { account: "foo", .. }))
        .times(1..)
        .return_const(());

    let notifier: Box<dyn Notifier> = Box::new(notifier);

    with_observer(Duration::from_millis(10), notifier, move |observer| {
        observer.add_account(account).unwrap();
        observer.logout_account("foo").unwrap();
        observer.add_account(account2).unwrap();
        std::thread::sleep(Duration::from_secs(1));
    });
}

#[test]
fn removing_account_produces_remove_notification() {
    let (_, account) = new_backend_and_account();

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

    with_observer(Duration::from_millis(10), notifier, move |observer| {
        observer.add_account(account).unwrap();
        observer.remove_account("foo").unwrap();
    });
}

#[test]
fn test_get_set_poll_interval() {
    let notifier: Box<dyn Notifier> = Box::new(NullNotifier {});
    let start_poll_interval = Duration::from_millis(10);

    with_observer(start_poll_interval, notifier, move |observer| {
        {
            let current_interval = observer.get_poll_interval().unwrap();
            assert_eq!(current_interval, start_poll_interval);
        }
        {
            let new_poll_interval = Duration::from_secs(20);
            observer.set_poll_interval(new_poll_interval).unwrap();
            let current_interval = observer.get_poll_interval().unwrap();
            assert_eq!(current_interval, new_poll_interval);
        }
    });
}

#[test]
fn test_proxy_settings() {
    // Apply the same proxy twice in a row and then remove it.
    let (_, account) = new_backend_and_account();

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
        .withf(|n| matches!(n, Notification::NewEmail { account: "foo", .. }))
        .times(1..)
        .return_const(());

    let notifier: Box<dyn Notifier> = Box::new(notifier);

    with_observer(Duration::from_millis(10), notifier, move |observer| {
        let proxy = Proxy {
            protocol: ProxyProtocol::Https,
            auth: None,
            url: "127.0.0.1".into(),
            port: 1080,
        };
        observer.add_account(account).unwrap();
        observer
            .set_proxy_settings("foo".to_string(), Some(proxy.clone()))
            .unwrap();
        observer
            .set_proxy_settings("foo".to_string(), Some(proxy.clone()))
            .unwrap();
        observer
            .set_proxy_settings("foo".to_string(), None)
            .unwrap();
        std::thread::sleep(Duration::from_secs(1));
    });
}

fn with_observer<F, T>(poll_interval: Duration, notifier: Box<dyn Notifier>, f: F)
where
    F: FnOnce(Observer) -> T,
{
    let observer = ObserverBuilder::new(notifier)
        .poll_interval(poll_interval)
        .build();

    (f)(observer.clone());

    observer.shutdown_worker().unwrap();
}
