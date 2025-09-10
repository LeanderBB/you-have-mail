use proton_api::mocks::mockito;
use sqlite_watcher::watcher::Watcher;
use std::sync::Arc;
use tempfile::TempDir;
use you_have_mail_common::backend::Backend;
use you_have_mail_common::encryption::Key;
use you_have_mail_common::state::State;
use you_have_mail_common::yhm::Yhm;
use you_have_mail_http::url;

/// Test context to keep track of resources.
pub struct TestCtx {
    pub yhm: Yhm,
    _temp_dir: TempDir,
    pub server: mockito::Server,
    pub state: Arc<State>,
}

impl TestCtx {
    /// Create a new instance.
    pub fn new() -> Self {
        let encryption_key = Key::new();
        let dir = TempDir::with_prefix("yhm_test").unwrap();
        let db_path = dir.path().join("sqlite.db");
        let server = proton_api::mocks::new_server();
        let watcher = Watcher::new().unwrap();
        let state = State::new(db_path, encryption_key, watcher).unwrap();

        let url = url::Url::parse(&proton_api::mocks::server_url(&server)).unwrap();
        tracing::info!("Mock Server: {}", url.to_string());

        let backend: Arc<dyn Backend> =
            you_have_mail_common::backend::proton::Backend::new(Some(url));
        let yhm = Yhm::with_backends(Arc::clone(&state), [backend]);

        Self {
            yhm,
            _temp_dir: dir,
            server,
            state,
        }
    }
}
