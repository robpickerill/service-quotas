use std::time::Duration;

use aws_config::{
    meta::region::RegionProviderChain,
    retry::{RetryConfig, RetryMode},
    SdkConfig,
};
use aws_sdk_cloudwatch::Region;

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

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_aws_config_with_region() {
        let (config, _) = aws_config_with_region("us-east-1").await;
        assert_eq!(config.region().unwrap().as_ref(), "us-east-1");
    }

    #[test]
    fn test_retry_config() {
        let retry_config = retry_config();
        assert_eq!(retry_config.initial_backoff(), Duration::new(2, 0));
        assert_eq!(retry_config.mode(), RetryMode::Adaptive);
        assert_eq!(retry_config.max_attempts(), 5);
    }
}
