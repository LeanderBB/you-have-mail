use log4rs::append::rolling_file::policy::compound::roll::fixed_window::FixedWindowRoller;
use log4rs::append::rolling_file::policy::compound::trigger::size::SizeTrigger;
use log4rs::append::rolling_file::policy::compound::CompoundPolicy;
use log4rs::append::rolling_file::RollingFileAppender;
use log4rs::config::{Appender, Logger, Root};
use log4rs::encode::pattern::PatternEncoder;
use std::path::PathBuf;
use std::sync::Once;
use uniffi::deps::anyhow;
use uniffi::deps::anyhow::anyhow;
use uniffi::deps::log::{info, LevelFilter};

static INIT_LOG_ONCE: Once = Once::new();

pub fn init_log(filepath: String) -> anyhow::Result<()> {
    let mut result = Ok(());

    let result_ref = &mut result;
    INIT_LOG_ONCE.call_once(move || {
        *result_ref = init_log_fn(filepath.into());
    });

    result
}

fn init_log_fn(filepath: PathBuf) -> anyhow::Result<()> {
    let pattern = "{d(%Y-%m-%d %H:%M:%S)} | {({l}):5.5} | {m}{n}";
    let log_file = RollingFileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(pattern)))
        .append(true)
        .build(
            filepath.join("yhm.log"),
            Box::new(CompoundPolicy::new(
                Box::new(SizeTrigger::new(5 * 1024 * 1024)),
                Box::new(
                    FixedWindowRoller::builder()
                        .base(0)
                        .build(&filepath.join("yhm.{}.log").to_string_lossy(), 2)
                        .map_err(|e| anyhow!("Failed to init window roller: {e}"))?,
                ),
            )),
        )
        .map_err(|e| anyhow!("Failed to build file logger: {e}"))?;

    let config = log4rs::Config::builder()
        .appender(Appender::builder().build("logfile", Box::new(log_file)))
        .logger(
            Logger::builder()
                .additive(false)
                .appender("logfile")
                .build("you_have_mail_common", LevelFilter::Debug),
        )
        .logger(
            Logger::builder()
                .additive(false)
                .appender("logfile")
                .build("youhavemail", LevelFilter::Debug),
        )
        .logger(
            Logger::builder()
                .additive(false)
                .appender("logfile")
                .build("youhavemail::bindings", LevelFilter::Error),
        )
        .build(Root::builder().appender("logfile").build(LevelFilter::Info))
        .map_err(|e| anyhow!("Failed to build log config: {e}"))?;

    log4rs::init_config(config).map_err(|e| anyhow!("Failed to init logger: {e}"))?;
    info!("Log file initialized");
    Ok(())
}
