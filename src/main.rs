use std::sync::{Arc, Mutex};

use tokio::sync::Semaphore;

mod cloudwatch;
mod quota;
mod service;
mod util;

#[tokio::main]
async fn main() {
    let services = service::list_service_codes().await;

    let mut handlers = Vec::with_capacity(services.len());
    let permits = Arc::new(Semaphore::new(10));

    let mut breached_quotas = Arc::new(Mutex::new(Vec::new()));
    for service in services {
        let permits = Arc::clone(&permits);
        let breached_quotas = Arc::clone(&breached_quotas);
        let handler = tokio::spawn(async move {
            let _permit = permits.acquire().await.unwrap();
            let _breached_quotas = breached_quotas.clone();

            let client = service::Client::new(75).await;
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

    for handler in handlers {
        let result = handler.await;
        match result {
            Err(err) => println!("{}", err),
            _ => (),
        }
    }

    for quota in breached_quotas.lock().unwrap().iter() {
        println!("{:?}", quota);
    }
    // let handle1 = tokio::spawn(async move {
    //     let client = service::Client::new().await;
    //     println!("{:?}", client.quotas("sagemaker").await);
    // });

    // let handle2 = tokio::spawn(async move {
    //     let client = service::Client::new().await;
    //     println!("{:?}", client.quotas("cloudformation").await);
    // });

    // let handle3 = tokio::spawn(async move {
    //     let client = service::Client::new().await;
    //     println!("{:?}", client.quotas("dynamodb").await);
    // });

    // handle1.await.unwrap();
    // handle2.await.unwrap();
    // handle3.await.unwrap();
}
