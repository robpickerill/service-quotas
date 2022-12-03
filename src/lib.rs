mod cli;
mod notifiers;
mod quota;
mod services;
mod util;

use async_mutex::Mutex;
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
    let permits = Arc::new(Semaphore::new(5));

    let breached_quotas = Arc::new(Mutex::new(Vec::new()));

    for region in regions {
        let services = services::servicequota::list_service_codes(&region.clone()).await;

        for service in services {
            let permits = Arc::clone(&permits);
            let breached_quotas = Arc::clone(&breached_quotas);

            let region_ = region.clone();
            let handler = tokio::spawn(async move {
                let _permit = permits.acquire().await.unwrap();
                let _breached_quotas = breached_quotas.clone();

                let client = services::servicequota::Client::new(&region_, threshold).await;
                let quotas = client.breached_quotas(&service).await;

                match quotas {
                    Err(err) => error!("{} quota lookup failed:{}", service, err),
                    Ok(results) => {
                        for result in results {
                            _breached_quotas.lock().await.push(result)
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

    for quota in breached_quotas.lock().await.iter() {
        info!("{:?}", quota);
    }

    if let Some(pd_key) = lift_pagerduty_routing_key() {
        let pagerduty = notifiers::pagerduty::Client::new(&pd_key, threshold)?;
        notify(pagerduty, breached_quotas).await;
    }

    Ok(())
}
