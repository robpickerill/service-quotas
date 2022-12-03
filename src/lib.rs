pub mod cli;

mod notifiers;
mod quota;
mod services;
mod util;

use async_mutex::Mutex;
use services::servicequota;
use std::sync::Arc;
use tokio::sync::Semaphore;

#[macro_use]
extern crate log;

fn lift_pagerduty_routing_key() -> Option<String> {
    std::env::var("PAGERDUTY_ROUTING_KEY").ok()
}

async fn notify(
    notifier: impl notifiers::Notifier,
    breached_quotas: Arc<Mutex<Vec<quota::Quota>>>,
) {
    for quota in breached_quotas.lock().await.iter() {
        let result = notifier.notify(quota.clone()).await;

        if let Err(err) = result {
            println!("pagerduty error: {}", err)
        }
    }
}

fn get_regions(args: &clap::ArgMatches) -> Vec<String> {
    let regions = args
        .get_many::<String>("regions")
        .unwrap()
        .cloned()
        .collect::<Vec<_>>();

    if regions.is_empty() {
        vec!["us-east-1".to_string()]
    } else {
        regions
    }
}

fn get_threshold(args: &clap::ArgMatches) -> u8 {
    args.get_one::<u8>("threshold").unwrap_or(&75).to_owned()
}

pub async fn run(args: &clap::ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let regions = get_regions(args);
    let threshold = get_threshold(args);

    let mut handlers = Vec::new();
    let all_quotas = Arc::new(Mutex::new(Vec::new()));

    for region in regions {
        info!("checking for quotas in region {}", region);

        let client = servicequota::Client::new(&region).await;
        let service_codes = client.service_codes().await?;
        let permits = Arc::new(Semaphore::new(3));

        for service_code in service_codes {
            let region_ = region.clone();

            let permits = Arc::clone(&permits);
            let client_ = client.clone();
            let all_quotas = Arc::clone(&all_quotas);

            let handler = tokio::spawn(async move {
                let _permit = permits.acquire().await.unwrap();
                let _all_quotas = all_quotas.clone();

                debug!(
                    "checking quotas region: {}, service: {}",
                    region_, service_code
                );

                let quotas = client_.quotas(&service_code).await;

                match quotas {
                    Err(err) => error!("{} quota lookup failed:{}", service_code, err),
                    Ok(results) => {
                        for result in results {
                            all_quotas.lock().await.push(result)
                        }
                    }
                }
            });
            handlers.push(handler)
        }
    }

    for handler in handlers {
        let result = handler.await;

        if let Err(err) = result {
            error!("error: {}", err)
        }
    }

    for quota in all_quotas.lock().await.iter() {
        if quota.utilization() > Some(threshold) {
            info!(
                "{:15}: {:30} {:12} {:30} : {:3}%",
                quota.region(),
                quota.service_code(),
                quota.quota_code(),
                quota.name(),
                quota.utilization().unwrap()
            )
        }
    }

    if let Some(pd_key) = lift_pagerduty_routing_key() {
        let pagerduty = notifiers::pagerduty::Client::new(&pd_key, threshold)?;
        notify(pagerduty, all_quotas).await;
    }

    Ok(())
}
