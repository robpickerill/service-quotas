use crate::services::{self, servicequota};
use crate::Args;

use std::sync::{Arc, Mutex};
use tokio::sync::Semaphore;

async fn resolve_regions(regions: Vec<String>) -> Vec<String> {
    if regions.is_empty() {
        return services::ec2::Client::new("us-east-1")
            .await
            .regions()
            .await;
    }

    regions
}

pub async fn run(args: Args) {
    let mut handlers = Vec::new();
    let permits = Arc::new(Semaphore::new(5));

    let regions = resolve_regions(args.regions).await;
    let breached_quotas = Arc::new(Mutex::new(Vec::new()));

    for region in regions {
        let services = servicequota::list_service_codes(&region.clone()).await;

        for service in services {
            let permits = Arc::clone(&permits);
            let breached_quotas = Arc::clone(&breached_quotas);

            let region_ = region.clone();
            let handler = tokio::spawn(async move {
                let _permit = permits.acquire().await.unwrap();
                let _breached_quotas = breached_quotas.clone();

                let client = servicequota::Client::new(&region_, args.threshold).await;
                let quotas = client.breached_quotas(&service).await;

                match quotas {
                    Err(err) => println!("{} quota lookup failed:{}", service, err),
                    Ok(results) => {
                        for result in results {
                            _breached_quotas.lock().unwrap().push(result)
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
            println!("error: {}", err)
        }
    }

    for quota in breached_quotas.lock().unwrap().iter() {
        println!("{:?}", quota);
    }
}
