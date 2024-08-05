use std::error::Error;
use std::path::PathBuf;
use std::sync::Once;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;
use uniffi::export;

static INIT_LOG_ONCE: Once = Once::new();

/// Initialize the log at `filepath`.
#[export]
pub fn init_log(filepath: String) -> Option<String> {
    let mut result = None;

    let result_ref = &mut result;
    INIT_LOG_ONCE.call_once(move || {
        *result_ref = match init_log_fn(filepath.into()) {
            Ok(()) => None,
            Err(e) => Some(e.to_string()),
        }
    });

    result
}

fn init_log_fn(path: PathBuf) -> Result<(), Box<dyn Error + Send + Sync>> {
    let appender = tracing_appender::rolling::never(path, "yhm.log");
    let filter = EnvFilter::builder().parse_lossy(
        "info,you_have_mail_common=debug,http=debug,proton_api=debug",
    );
    tracing_subscriber::FmtSubscriber::builder()
        .with_ansi(false)
        .with_writer(appender)
        .with_max_level(LevelFilter::DEBUG)
        .with_env_filter(filter)
        .try_init()
}

#[export]
fn yhm_log_info(text: &str) {
    tracing::info!("[APP] {text}");
}

#[export]
fn yhm_log_error(text: &str) {
    tracing::error!("[APP] {text}");
}

#[export]
fn yhm_log_warn(text: &str) {
    tracing::warn!("[APP] {text}");
}
