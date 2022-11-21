use std::sync::Arc;

use tokio::sync::Semaphore;

mod cloudwatch;
mod quota;
mod service;
mod util;

#[tokio::main]
async fn main() {
    let services = service::list_service_codes().await;

    let mut handlers = Vec::with_capacity(services.len());
    let permits = Arc::new(Semaphore::new(5));

    for service in services {
        let permits = Arc::clone(&permits);
        let handler = tokio::spawn(async move {
            let _permit = permits.acquire().await.unwrap();

            let client = service::Client::new().await;
            let result = client.quotas(&service).await;

            match result {
                Err(err) => println!("{} quota lookup failed:{}", service, err),
                Ok(result) => println!("{:?}", result),
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

    // let handle1 = tokio::spawn(async move {
    //     let client = service::Client::new().await;
    //     println!("{:?}", client.quotas("ec2").await);
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
