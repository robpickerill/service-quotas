pub mod pagerduty;

use crate::quotas::Quota;
use async_trait::async_trait;
use std::error::Error;

#[async_trait]
pub trait Notify: Send + Sync {
    async fn notify(&self, quota: &[Box<dyn Quota>]) -> Result<(), Box<dyn Error>>;
}

pub async fn lookup_notifiers(
    threshold: &u8,
    ignored_quotas: Option<&[String]>,
) -> Result<Option<Box<impl Notify>>, Box<dyn Error>> {
    // Pagerduty Notifier: sourced from the PAGERDUTY_ROUTING_KEY environment variable.
    if let Some(routing_key) = pd_routing_key() {
        let pd_client = pagerduty::Client::new(&routing_key, threshold, ignored_quotas)?;
        return Ok(Some(Box::new(pd_client)));
    }

    Ok(None)
}

fn pd_routing_key() -> Option<String> {
    if let Ok(routing_key) = std::env::var("PAGERDUTY_ROUTING_KEY") {
        return Some(routing_key);
    }

    None
}
