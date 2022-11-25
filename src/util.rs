use std::time::Duration;

use aws_config::{
    meta::region::RegionProviderChain,
    retry::{RetryConfig, RetryMode},
    SdkConfig,
};
use aws_sdk_cloudwatch::Region;

// aws_config loads aws configurations for use with aws clients, lifting the region from the
// environment variable: AWS_REGION
pub async fn aws_config() -> (SdkConfig, RetryConfig) {
    (aws_config::load_from_env().await, retry_config())
}

// aws_config_with_region loads aws configurations for a specific region for use with aws clients
pub async fn aws_config_with_region(region: &str) -> (SdkConfig, RetryConfig) {
    let region_provider = RegionProviderChain::first_try(Region::new(region.to_string()));
    (
        aws_config::from_env().region(region_provider).load().await,
        retry_config(),
    )
}

fn retry_config() -> RetryConfig {
    RetryConfig::standard()
        .with_initial_backoff(Duration::new(2, 0))
        .with_retry_mode(RetryMode::Adaptive)
        .with_max_attempts(5)
}
