use aws_config::{
    retry::{RetryConfig, RetryMode},
    SdkConfig,
};

// aws_config loads aws configurations for use with aws clients
pub async fn aws_config() -> (SdkConfig, RetryConfig) {
    (
        aws_config::load_from_env().await,
        RetryConfig::standard().with_retry_mode(RetryMode::Adaptive),
    )
}
