use crate::backend::null::NullTestAccount;
use crate::backend::Backend;
use crate::observer::{Observer, ObserverBuilder};
use crate::{Account, ConfigAuthRefresher, Notifier, Proxy, ProxyProtocol};
use crate::{Config, Notification};
use secrecy::SecretString;
use std::sync::Arc;
use std::time::Duration;

use crate::MockNotifier;
use mockall::Sequence;

fn new_backend_and_account(refresh: bool) -> (Arc<dyn Backend>, Account) {
    let accounts = NullTestAccount {
        email: "foo".to_string(),
        password: "bar".to_string(),
        totp: None,
        wait_time: None,
        refresh,
    };
    let backend = crate::backend::null::new_backend(&[accounts]);
    let mut account = Account::new(backend.clone(), "foo", None);
    account
        .login(&SecretString::new("bar".into()), None)
        .unwrap();

    assert!(account.is_logged_in());
    (backend, account)
}

#[test]
fn notifier_called() {
    let (_, account) = new_backend_and_account(false);

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

    let notifier: Arc<dyn Notifier> = Arc::new(notifier);

    with_observer(Duration::from_millis(50), notifier, move |mut observer| {
        observer.add_account(account).unwrap();
        observer.poll_foreground().expect("failed to poll");
    });
}

#[test]
fn adding_account_after_logout_works() {
    let (_, account) = new_backend_and_account(false);
    let (_, account2) = new_backend_and_account(false);

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
        .withf(|n| matches!(n, Notification::NewEmail { account: "foo", .. }))
        .times(1..)
        .return_const(());

    let notifier: Arc<dyn Notifier> = Arc::new(notifier);

    with_observer(Duration::from_millis(10), notifier, move |mut observer| {
        observer.add_account(account).unwrap();
        observer.logout_account("foo").unwrap();
        observer.add_account(account2).unwrap();
        observer.poll_foreground().expect("failed to poll");
    });
}

#[test]
fn removing_account_produces_remove_notification() {
    let (_, account) = new_backend_and_account(false);

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

    let notifier: Arc<dyn Notifier> = Arc::new(notifier);

    with_observer(Duration::from_millis(10), notifier, move |mut observer| {
        observer.add_account(account).unwrap();
        observer.remove_account("foo").unwrap();
    });
}

#[test]
fn test_proxy_settings() {
    // Apply the same proxy twice in a row and then remove it.
    let (_, account) = new_backend_and_account(false);

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

    let notifier: Arc<dyn Notifier> = Arc::new(notifier);

    with_observer(Duration::from_millis(10), notifier, move |mut observer| {
        let proxy = Proxy {
            protocol: ProxyProtocol::Https,
            auth: None,
            url: "127.0.0.1".into(),
            port: 1080,
        };
        observer.add_account(account).unwrap();
        observer
            .set_proxy_settings("foo".to_string(), Some(&proxy))
            .unwrap();
        observer
            .set_proxy_settings("foo".to_string(), Some(&proxy))
            .unwrap();
        observer
            .set_proxy_settings("foo".to_string(), None)
            .unwrap();
        observer.poll_foreground().expect("failed to poll");
    });
}

#[test]
fn test_account_refreshed() {
    // Apply the same proxy twice in a row and then remove it.
    let (_, account) = new_backend_and_account(true);

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
        .withf(|n| matches!(n, Notification::NewEmail { account: "foo", .. }))
        .times(1..)
        .return_const(());

    let notifier: Arc<dyn Notifier> = Arc::new(notifier);

    with_observer(Duration::from_millis(10), notifier, move |mut observer| {
        let account_refresh_data_start = account
            .get_impl()
            .expect("invalid state")
            .to_refresher()
            .to_json()
            .expect("failed to serialize");
        observer.add_account(account).unwrap();
        observer.poll_foreground().expect("failed to poll");
        observer.config().read(|inner| {
            let cfg_account = inner.get_account("foo").expect("failed to locate account");
            let ConfigAuthRefresher::Resolved(r) = &cfg_account.auth_refresher  else {
                panic!("unexpected state");
            };
            let value = r.to_json().expect("failed to serialize");
            let account_refresh_data_current = observer
                .get_account("foo")
                .expect("Failed to locate account")
                .get_impl()
                .expect("invalid state")
                .to_refresher()
                .to_json()
                .expect("failed to serialize");
            assert_ne!(account_refresh_data_start, value);
            assert_eq!(account_refresh_data_current, value);
        })
    });
}

fn with_observer<F, T>(poll_interval: Duration, notifier: Arc<dyn Notifier>, f: F)
where
    F: FnOnce(Observer) -> T,
{
    use crate::EncryptionKey;

    let tmp_dir = temp_dir::TempDir::new().expect("failed to create tmp dir");
    let encryption_key = EncryptionKey::new();
    let config_path = tmp_dir.child("config");

    let config =
        Config::create_or_load(encryption_key, config_path).expect("failed to load config");

    let observer = ObserverBuilder::new(notifier, config)
        .poll_interval(poll_interval)
        .build()
        .expect("Failed to build observer");

    (f)(observer);
}
