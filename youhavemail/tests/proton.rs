mod common;

use crate::common::TestCtx;
use proton_api::auth::{Auth, RefreshToken, Token, Uid};
use proton_api::domain::event::MoreEvents;
use proton_api::domain::{event, label, message, Boolean, SecretString};
use secrecy::ExposeSecret;
use you_have_mail_common::backend::proton::TaskState;
use you_have_mail_common::state::Account;

#[test]
fn login_sequence() {
    // check if the account is created correctly.
    let mut ctx = TestCtx::new();
    let backend = ctx
        .yhm
        .backend_with_name(you_have_mail_common::backend::proton::PROTON_BACKEND_NAME)
        .unwrap()
        .clone();

    let client = backend.create_client(None).unwrap();
    let session = proton_api::session::Session::with_in_memory_auth_store(client);
    let mut sequence = proton_api::login::Sequence::without_server_proof_check(session);

    assert_eq!(ctx.yhm.account_count().unwrap(), 0);
    {
        let _auth_mocks = proton_api::mocks::auth::login_flow(&mut ctx.server, false);
        sequence
            .login(
                proton_api::mocks::DEFAULT_USER_EMAIL,
                proton_api::mocks::DEFAULT_USER_PASSWORD,
                None,
            )
            .unwrap();
        ctx.yhm.add(sequence).unwrap();
    }

    assert_eq!(ctx.yhm.account_count().unwrap(), 1);

    assert!(account_state(&ctx).is_none());
    let auth = account_auth(&ctx).unwrap();

    assert_eq!(
        auth.uid,
        Uid(proton_api::mocks::auth::SESSION_UID.to_owned())
    );
    assert_eq!(
        auth.auth_token.0.expose_secret(),
        proton_api::mocks::auth::ACCESS_TOKEN
    );
    assert_eq!(
        auth.refresh_token.0.expose_secret(),
        proton_api::mocks::auth::REFRESH_TOKEN
    );
}
#[test]
fn poll_sequence() {
    // Test basic flow when polling an account, from basic initialization to subsequent runs.
    let mut ctx = TestCtx::new();
    create_authenticated_account(&ctx, None);

    let event_id = event_id(1);
    let event = event::Event {
        event_id: event_id.clone(),
        more: MoreEvents::No,
        messages: None,
        labels: None,
    };

    let label_id_with_notification = label::Id("label".to_owned());
    let label_id_without_notification = label::Id("label_silent".to_owned());

    let labels = vec![
        label::Label {
            id: label_id_with_notification.clone(),
            parent_id: None,
            name: "".to_string(),
            path: "".to_string(),
            color: "".to_string(),
            label_type: label::Type::Folder,
            notify: Boolean::True,
            display: Default::default(),
            sticky: Default::default(),
            expanded: Default::default(),
            order: 0,
        },
        label::Label {
            id: label_id_without_notification.clone(),
            parent_id: None,
            name: "".to_string(),
            path: "".to_string(),
            color: "".to_string(),
            label_type: label::Type::Folder,
            notify: Boolean::False,
            display: Default::default(),
            sticky: Default::default(),
            expanded: Default::default(),
            order: 0,
        },
    ];

    // First time, no state. Things need to be fetched.
    {
        let _mock_latest_event =
            proton_api::mocks::events::get_latest_event_id(&mut ctx.server, event_id.clone());
        let _mock_labels =
            proton_api::mocks::labels::get_labels(&mut ctx.server, label::Type::Folder, &labels);

        let _mock_event =
            proton_api::mocks::events::get_event(&mut ctx.server, &event.event_id, &event);

        let mut result = ctx.yhm.poll().unwrap();
        let output = result.remove(0);

        assert_eq!(output.email, ACCOUNT_EMAIL);
        assert_eq!(
            output.backend,
            you_have_mail_common::backend::proton::PROTON_BACKEND_NAME
        );
        let new_emails = output.result.unwrap();
        assert!(new_emails.is_empty());

        // state should now be saved and have the extra notifiable folder.
        let state = account_state(&ctx).expect("account should have state");
        assert_eq!(state.last_event_id, Some(event_id.clone()));
        assert_eq!(state.active_folder_ids.len(), 2);
        assert!(state
            .active_folder_ids
            .contains(&label_id_with_notification));
        assert!(state.active_folder_ids.contains(&label::Id::inbox()));
    }

    // 2nd time state, only fetch events.
    {
        let _mock_event =
            proton_api::mocks::events::get_event(&mut ctx.server, &event.event_id, &event);
        let mut result = ctx.yhm.poll().unwrap();
        let output = result.remove(0);

        assert_eq!(output.email, ACCOUNT_EMAIL);
        assert_eq!(
            output.backend,
            you_have_mail_common::backend::proton::PROTON_BACKEND_NAME
        );
        let new_emails = output.result.unwrap();
        assert!(new_emails.is_empty());

        // No changes have been made to the state.
        let state = account_state(&ctx).expect("account should have state");
        assert_eq!(state.last_event_id, Some(event_id));
        assert_eq!(state.active_folder_ids.len(), 2);
        assert!(state
            .active_folder_ids
            .contains(&label_id_with_notification));
        assert!(state.active_folder_ids.contains(&label::Id::inbox()));
    }
}

#[test]
fn poll_event_loop() {
    // check event loop logic,
    let mut ctx = TestCtx::new();

    let event_id0 = event_id(0);
    let event_id1 = event_id(1);
    let event_id2 = event_id(2);
    let event_id3 = event_id(3);

    let event_1 = event::Event {
        event_id: event_id1.clone(),
        more: MoreEvents::Yes,
        messages: None,
        labels: None,
    };

    let event_2 = event::Event {
        event_id: event_id2.clone(),
        more: MoreEvents::Yes,
        messages: None,
        labels: None,
    };

    let event_3 = event::Event {
        event_id: event_id3.clone(),
        more: MoreEvents::No,
        messages: None,
        labels: None,
    };

    create_authenticated_account(&ctx, Some(TaskState::with_event_id(event_id0.clone())));

    {
        let _event_1_mock =
            proton_api::mocks::events::get_event(&mut ctx.server, &event_id0, &event_1);
        let _event_2_mock =
            proton_api::mocks::events::get_event(&mut ctx.server, &event_id1, &event_2);
        let _event_3_mock =
            proton_api::mocks::events::get_event(&mut ctx.server, &event_id2, &event_3);
        let _event_4_mock =
            proton_api::mocks::events::get_event(&mut ctx.server, &event_id3, &event_3);

        let output = ctx.yhm.poll().unwrap().remove(0);
        let info = output.result.unwrap();
        assert!(info.is_empty());
    }

    let state = account_state(&ctx).expect("account should have state");
    assert_eq!(state.last_event_id, Some(event_id3));
}

#[test]
fn message_event_creates_notification() {
    let mut ctx = TestCtx::new();

    let event_id0 = event_id(0);
    let event_id1 = event_id(1);
    let message_id = message::Id("message".to_owned());

    let subject = "hello world!".to_owned();
    let sender_address = "bar@proton.me".to_owned();

    let event_1 = event::Event {
        event_id: event_id1.clone(),
        more: MoreEvents::No,
        messages: Some(vec![event::Message {
            id: message_id.clone(),
            action: event::Action::Create,
            message: Some(message::Message {
                id: message_id.clone(),
                labels: vec![label::Id::inbox()],
                subject: subject.clone(),
                sender_address: sender_address.clone(),
                sender_name: None,
                unread: Boolean::True,
            }),
        }]),
        labels: None,
    };

    let event_loop_exit = event::Event {
        event_id: event_id1.clone(),
        more: MoreEvents::No,
        messages: None,
        labels: None,
    };

    create_authenticated_account(&ctx, Some(TaskState::with_event_id(event_id0.clone())));

    {
        let _event_1_mock =
            proton_api::mocks::events::get_event(&mut ctx.server, &event_id0, &event_1);
        let _event_2_mock =
            proton_api::mocks::events::get_event(&mut ctx.server, &event_id1, &event_loop_exit);

        let output = ctx.yhm.poll().unwrap().remove(0);
        let info = output.result.unwrap();
        assert!(!info.is_empty());
        assert_eq!(info[0].subject, subject);
        assert_eq!(info[0].sender, sender_address);
    }

    let state = account_state(&ctx).expect("account should have state");
    assert_eq!(state.last_event_id, Some(event_id1));
}

#[test]
fn no_poll_after_logout() {
    let mut ctx = TestCtx::new();
    create_authenticated_account(&ctx, None);

    {
        let _mock = proton_api::mocks::auth::logout(&mut ctx.server);
        ctx.yhm.logout(ACCOUNT_EMAIL).unwrap();
    }

    assert_eq!(ctx.yhm.account_count().unwrap(), 1);
    assert!(ctx.yhm.poll().unwrap().is_empty());
}

#[test]
fn no_poll_after_delete() {
    let mut ctx = TestCtx::new();
    create_authenticated_account(&ctx, None);

    {
        let _mock = proton_api::mocks::auth::logout(&mut ctx.server);
        ctx.yhm.delete(ACCOUNT_EMAIL).unwrap();
    }

    assert!(ctx.yhm.poll().unwrap().is_empty());
    assert_eq!(ctx.yhm.account_count().unwrap(), 0);
}

fn create_authenticated_account(ctx: &TestCtx, state: Option<TaskState>) {
    let mut account = Account::new(
        ACCOUNT_EMAIL.to_string(),
        you_have_mail_common::backend::proton::PROTON_BACKEND_NAME.to_string(),
    );
    let auth = proton_api::auth::Auth {
        uid: Uid(proton_api::mocks::session_id().to_owned()),
        auth_token: Token(SecretString::new(
            proton_api::mocks::auth::ACCESS_TOKEN.to_owned(),
        )),
        refresh_token: RefreshToken(SecretString::new(
            proton_api::mocks::auth::REFRESH_TOKEN.to_owned(),
        )),
    };

    account.set_state(state.as_ref()).unwrap();
    account
        .set_secret(ctx.state.encryption_key().expose_secret(), Some(&auth))
        .unwrap();
    ctx.yhm.add(account).expect("Failed to add account");
}

fn account_state(ctx: &TestCtx) -> Option<TaskState> {
    let account = ctx
        .state
        .account(ACCOUNT_EMAIL)
        .unwrap()
        .expect("failed to find account");
    account.state::<TaskState>().unwrap()
}

fn account_auth(ctx: &TestCtx) -> Option<Auth> {
    let account = ctx
        .state
        .account(ACCOUNT_EMAIL)
        .unwrap()
        .expect("failed to find account");
    account
        .secret::<Auth>(ctx.state.encryption_key().expose_secret())
        .unwrap()
}

fn event_id(id: u32) -> event::Id {
    event::Id(id.to_string())
}

const ACCOUNT_EMAIL: &str = proton_api::mocks::DEFAULT_USER_EMAIL;
