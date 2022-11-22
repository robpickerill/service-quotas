use std::sync::{Arc, Mutex};

use clap::Parser;
use tokio::sync::Semaphore;

mod cloudwatch;
mod quota;
mod service;
mod util;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// the threshold by which to alert on for utilization of a service quota
    #[arg(short, long, default_value_t = 75)]
    threshold: u8,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    run(args).await;
}

async fn run(args: Args) {
    let services = service::list_service_codes().await;

    let mut handlers = Vec::with_capacity(services.len());
    let permits = Arc::new(Semaphore::new(5));

    let breached_quotas = Arc::new(Mutex::new(Vec::new()));

    for service in services {
        let permits = Arc::clone(&permits);
        let breached_quotas = Arc::clone(&breached_quotas);

        let handler = tokio::spawn(async move {
            let _permit = permits.acquire().await.unwrap();
            let _breached_quotas = breached_quotas.clone();

            let client = service::Client::new(args.threshold).await;
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

        if let Err(err) = result {
            println!("error: {}", err)
        }
    }

    for quota in breached_quotas.lock().unwrap().iter() {
        println!("{:?}", quota);
    }
}
