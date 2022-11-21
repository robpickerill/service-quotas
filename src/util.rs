use std::time::Duration;

use aws_config::{
    retry::{RetryConfig, RetryMode},
    SdkConfig,
};

// aws_config loads aws configurations for use with aws clients
pub async fn aws_config() -> (SdkConfig, RetryConfig) {
    (
        aws_config::load_from_env().await,
        RetryConfig::standard()
            .with_initial_backoff(Duration::new(2, 0))
            .with_retry_mode(RetryMode::Adaptive)
            .with_max_attempts(5),
    )
}
