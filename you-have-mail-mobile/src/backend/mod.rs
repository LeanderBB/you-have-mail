pub mod proton;

use std::sync::Arc;
use you_have_mail_common as yhm;

/// Represents a backend implementation.
#[derive(uniffi::Object)]
pub struct Backend(pub Arc<dyn yhm::backend::Backend>);

#[uniffi::export]
impl Backend {
    /// Get the backend name.
    #[must_use]
    pub fn name(&self) -> String {
        self.0.name().to_owned()
    }

    /// Get a short description about this backend.
    #[must_use]
    pub fn description(&self) -> String {
        self.0.description().to_owned()
    }
}
