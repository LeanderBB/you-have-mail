use anyhow::anyhow;
use log4rs::append::console::{ConsoleAppender, Target};
use log4rs::append::rolling_file::policy::compound::roll::fixed_window::FixedWindowRoller;
use log4rs::append::rolling_file::policy::compound::trigger::size::SizeTrigger;
use log4rs::append::rolling_file::policy::compound::CompoundPolicy;
use log4rs::append::rolling_file::RollingFileAppender;
use log4rs::config::{Appender, Logger, Root};
use log4rs::encode::pattern::PatternEncoder;
use proton_api_rs::log::LevelFilter;
use std::path::Path;

pub fn init_log(
    file_path: impl AsRef<Path>,
    debug_module_list: impl IntoIterator<Item = String>,
) -> Result<(), anyhow::Error> {
    let pattern = "{d(%Y-%m-%d %H:%M:%S)} | {({l}):5.5} | {m}{n}";
    let console = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new(pattern)))
        .target(Target::Stdout)
        .build();
    let log_file = RollingFileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(pattern)))
        .append(true)
        .build(
            file_path.as_ref().join("yhm.log"),
            Box::new(CompoundPolicy::new(
                Box::new(SizeTrigger::new(5 * 1024 * 1024)),
                Box::new(
                    FixedWindowRoller::builder()
                        .base(0)
                        .build(&file_path.as_ref().join("yhm.{}.log").to_string_lossy(), 2)
                        .map_err(|e| anyhow!("Failed to init window roller: {e}"))?,
                ),
            )),
        )
        .map_err(|e| anyhow!("Failed to build file logger: {e}"))?;

    let loggers = debug_module_list
        .into_iter()
        .map(|name| {
            Logger::builder()
                .additive(false)
                .appender("logfile")
                .build(name, LevelFilter::Debug)
        })
        .collect::<Vec<_>>();

    let config = log4rs::Config::builder()
        .appender(Appender::builder().build("console", Box::new(console)))
        .appender(Appender::builder().build("logfile", Box::new(log_file)))
        .logger(
            Logger::builder()
                .additive(false)
                .appender("logfile")
                .build("you_have_mail_common", LevelFilter::Debug),
        )
        .loggers(loggers)
        .build(
            Root::builder()
                .appender("console")
                .appender("logfile")
                .build(LevelFilter::Info),
        )
        .map_err(|e| anyhow!("Failed to build log config: {e}"))?;

    log4rs::init_config(config).map_err(|e| anyhow!("Failed to init logger: {e}"))?;
    Ok(())
}
