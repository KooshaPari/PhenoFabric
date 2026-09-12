//! Structured logging setup for fabric-daemon.

use crate::config::LoggingConfig;
use tracing_subscriber::EnvFilter;

/// Initialize structured logging.
pub fn init_logging(config: &LoggingConfig) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.level));

    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_thread_ids(true);

    match config.format.as_str() {
        "json" => {
            subscriber.json().init();
        }
        "compact" => {
            subscriber.compact().init();
        }
        _ => {
            // "pretty" or default
            subscriber.init();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_logging_does_not_panic() {
        let config = LoggingConfig {
            level: "off".into(),
            format: "pretty".into(),
            file: None,
        };
        // Should not panic (may warn if already initialized in same process).
        init_logging(&config);
    }
}
