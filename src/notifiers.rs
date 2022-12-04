pub mod pagerduty;

use crate::quota::Quota;
use async_trait::async_trait;
use std::error::Error;

#[async_trait]
pub trait Notifier {
    type Error: Error;

    async fn notify(&self, quota: &Quota) -> Result<(), Self::Error>;
}
