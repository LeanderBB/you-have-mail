use crate::auth::{Auth, InMemoryStore, ThreadSafeStore, new_thread_safe_store};
use crate::domain::user::User;
use crate::requests::{GetUserInfoRequest, LogoutRequest, PostAuthRefreshRequest};
use anyhow::anyhow;
use http::{Client, FromResponse, Method, Request, RequestBuilder};
use secrecy::ExposeSecret;
use std::sync::Arc;
use tracing::{error, warn};

/// Authenticated Session from which one can access data/functionality restricted to authenticated
/// users.
#[derive(Clone)]
pub struct Session {
    auth_store: ThreadSafeStore,
    client: Arc<Client>,
}

struct ProtonRequest<T: Request> {
    request: T,
}

impl<T: Request> Request for ProtonRequest<T> {
    type Response = T::Response;
    const METHOD: Method = T::METHOD;

    fn url(&self) -> String {
        self.request.url()
    }

    fn build(&self, mut builder: RequestBuilder) -> http::Result<RequestBuilder> {
        builder = builder.header(X_PM_APP_VERSION_HEADER, DEFAULT_APP_VERSION);
        self.request.build(builder)
    }
}

struct ProtonAuthRequest<'s, T: Request> {
    session: &'s Session,
    request: T,
}

impl<T: Request> Request for ProtonAuthRequest<'_, T> {
    type Response = T::Response;
    const METHOD: Method = T::METHOD;

    fn url(&self) -> String {
        self.request.url()
    }

    fn build(&self, mut builder: RequestBuilder) -> http::Result<RequestBuilder> {
        if let Some(auth) = self.session.auth_store.read().get().map_err(|e| {
            http::Error::Unexpected(anyhow::anyhow!("Failed to read authentication data: {e}"))
        })? {
            builder = builder.bearer_token(auth.auth_token.0.expose_secret());
            builder = builder.header(X_PM_UID_HEADER, auth.uid.as_ref());
        } else {
            warn!("Authenticated requested without authentication data");
        }
        builder = builder.header(X_PM_APP_VERSION_HEADER, DEFAULT_APP_VERSION);
        self.request.build(builder)
    }
}

impl Session {
    /// Create a new instance with a given `client` and `auth_store`.
    pub fn new(client: Arc<Client>, auth_store: ThreadSafeStore) -> Self {
        Self { auth_store, client }
    }

    /// Create a new instance with a given `client` and an [`crate::auth::InMemoryStore`].
    #[must_use]
    pub fn with_in_memory_auth_store(client: Arc<Client>) -> Self {
        Self {
            auth_store: new_thread_safe_store(InMemoryStore::default()),
            client,
        }
    }

    /// Get http client.
    #[must_use]
    pub fn client(&self) -> &Arc<Client> {
        &self.client
    }

    /// Get the sessions authentication store.
    #[must_use]
    pub fn auth_store(&self) -> &ThreadSafeStore {
        &self.auth_store
    }

    /// Get user info.
    ///
    /// # Errors
    /// Returns error if the request failed.
    pub fn user_info(&self) -> http::Result<User> {
        Ok(self.execute_with_auth(GetUserInfoRequest {})?.user)
    }

    /// Logout the session.
    ///
    /// # Errors
    /// Returns error if the request failed.
    pub fn logout(&self) -> http::Result<()> {
        self.execute_with_auth(LogoutRequest {})?;
        self.auth_store.write().delete().map_err(|e| {
            http::Error::Unexpected(anyhow!("Failed to delete authentication data: {e}"))
        })
    }

    /// Execute a non-authenticate request with this client.
    ///
    /// # Errors
    /// Returns error if the request failed.
    pub fn execute<T: Request>(
        &self,
        request: T,
    ) -> http::Result<<T::Response as FromResponse>::Output> {
        let request = ProtonRequest { request };

        match self.client.execute(&request) {
            Ok(v) => Ok(v),
            Err(e) => self.handle_error(&request, e),
        }
    }

    /// Execute an authenticate request with this client.
    ///
    /// Note that the session token is automatically refreshed and the request is retried
    /// on successful refresh.
    ///
    /// # Errors
    /// Returns error if the request  or accessing/updating the session token failed.
    pub fn execute_with_auth<T: Request>(
        &self,
        request: T,
    ) -> http::Result<<T::Response as FromResponse>::Output> {
        let request = ProtonAuthRequest {
            session: self,
            request,
        };

        match self.client.execute(&request) {
            Ok(v) => Ok(v),
            Err(e) => self.handle_error(&request, e),
        }
    }

    /// Check if this was a session expired error and attempt to auto refresh.
    fn handle_error<T: Request>(
        &self,
        request: &T,
        error: http::Error,
    ) -> http::Result<<T::Response as FromResponse>::Output> {
        let http::Error::Http(401, _) = &error else {
            return Err(error);
        };

        // Attempt refresh
        let mut guard = self.auth_store.write();

        let Ok(Some(auth)) = guard.get() else {
            error!("Failed to get authentication data");
            return Err(error);
        };

        let response = match self.execute(PostAuthRefreshRequest::new(
            &auth.uid,
            auth.refresh_token.0.expose_secret(),
        )) {
            Ok(response) => response,
            Err(e) => {
                if let Err(e) = guard.delete() {
                    error!("Failed to delete auth data after failed refresh: {e}");
                }
                error!("Failed to refresh auth token: {e}");
                return Err(error);
            }
        };

        if let Err(e) = guard.store(Auth {
            uid: response.uid,
            auth_token: response.access_token,
            refresh_token: response.refresh_token,
        }) {
            error!("Failed to update token in auth store: {e}");
            if let Err(e) = guard.delete() {
                error!("Failed to remove token from auth store after update failure: {e}");
            }
            return Err(error);
        }

        drop(guard);

        // Execute the request again.
        self.client.execute(request)
    }
}

pub(crate) const DEFAULT_APP_VERSION: &str = "Other";
pub(crate) const DEFAULT_HOST_URL: &str = "https://mail.proton.me/api/";
pub(crate) const X_PM_APP_VERSION_HEADER: &str = "X-Pm-Appversion";
pub(crate) const X_PM_UID_HEADER: &str = "X-Pm-Uid";
