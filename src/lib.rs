pub mod cli;

mod config;
mod quota;
mod services;
mod util;

#[macro_use]
extern crate prettytable;

use prettytable::{format, Cell, Row, Table};
use quota::Quota;
use services::servicequota;
use std::sync::Arc;
use tokio::sync::Semaphore;

pub async fn list_quotas() -> Result<(), Box<dyn std::error::Error>> {
    let client = servicequota::Client::new("us-east-1").await;
    let service_codes = client.service_codes().await.unwrap();

    // using a semaphore here to avoid rate limiting from the AWS APIs
    let permits = Arc::new(Semaphore::new(3));

    let tasks = service_codes
        .into_iter()
        .map(|s| {
            let client_ = client.clone();
            let permits = Arc::clone(&permits);
            tokio::spawn(async move {
                let _permits = permits.acquire().await.unwrap();
                client_.quotas(&s).await.unwrap()
            })
        })
        .collect::<Vec<_>>();

    let mut quotas = Vec::new();
    for task in tasks {
        match task.await {
            Ok(quotas_) => quotas.extend(quotas_),
            Err(err) => println!("error: {}", err),
        }
    }

    print_list_quotas_table(quotas).await;

    Ok(())
}

async fn utilization_per_service(
    client: &servicequota::Client,
    service_code: &str,
    permits: Arc<Semaphore>,
) -> Result<Vec<Quota>, Box<dyn std::error::Error + Send + Sync>> {
    let _permits = permits.acquire().await.unwrap();
    let quotas = client.quotas(service_code).await?;

    for quota in quotas.clone() {
        quota.utilization().await;
    }

    Ok(quotas)
}

pub async fn list_quotas(args: &clap::ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let config = config::Config::new(args);

    for region in config.regions() {
        let client = servicequota::Client::new(region).await;
        let service_codes = client.service_codes().await?;

        for service_code in service_codes {
            let quotas = client.quotas(&service_code).await?;

            for quota in quotas {
                if quota.enabled() {
                    println!("{:90} {:50}", quota.arn(), quota.name())
                }
            }
        }
    }

    Ok(())
}

fn lift_pagerduty_routing_key() -> Option<String> {
    std::env::var("PAGERDUTY_ROUTING_KEY").ok()
}

async fn notify(
    notifier: impl notifiers::Notifier,
    breached_quotas: &Vec<quota::Quota>,
    ignored_quotas: &HashSet<String>,
) {
    for quota in breached_quotas {
        if ignored_quotas.contains(&quota.quota_code().to_string()) {
            info!(
                "Ignoring quota {} as it is in the ignore list",
                quota.quota_code()
            );
            continue;
        }

        let result = notifier.notify(quota).await;

        if let Err(err) = result {
            println!("pagerduty error: {}", err)
        }
    }
}

fn log_startup(config: &config::Config) {
    info!(
        "Starting up: {} {}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );
    info!("Region: {}", config.regions().join(", "));
    info!("Threshold: {}", config.threshold());
    info!(
        "Ignored quotas: {}",
        config
            .ignored_quotas()
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
}

async fn log_breached_quotas(quotas: &Vec<Quota>, config: &config::Config) {
    let mut count = 0;

    for quota in quotas {
        table.add_row(Row::new(vec![
            Cell::new(quota.arn().await),
            Cell::new(quota.name().await),
        ]));
    }

    table.printstd();
}

pub async fn utilization() -> Result<(), Box<dyn std::error::Error>> {
    let client = servicequota::Client::new("us-east-1").await;
    let service_codes = client.service_codes().await.unwrap();

    // using a semaphore here to avoid rate limiting from the AWS APIs
    let permits = Arc::new(Semaphore::new(3));

    let tasks = service_codes
        .into_iter()
        .map(|s| {
            let client_ = client.clone();
            let permits = Arc::clone(&permits);
            tokio::spawn(async move {
                let _permits = permits.acquire().await.unwrap();
                client_.quotas(&s).await.unwrap()
            })
        })
        .collect::<Vec<_>>();

    let mut breached_quotas = Vec::new();
    for task in tasks {
        match task.await {
            Ok(quotas_) => {
                for quota in quotas_ {
                    if let Some(utilization) = quota.utilization().await {
                        if utilization >= 75 {
                            breached_quotas.push(quota);
                        }
                    }
                }
            }
            Err(err) => println!("error: {}", err),
        }
    }

    if breached_quotas.is_empty() {
        println!("No breached quotas found");
    } else {
        print_breached_quotas_table(breached_quotas).await;
    }

    Ok(())
}

async fn print_breached_quotas_table(quotas: Vec<Box<dyn Quota>>) {
    let mut table = Table::new();
    table.add_row(row!["ARN", "Quota Name", "Utilization"]);
    table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);

    for quota in quotas {
        table.add_row(Row::new(vec![
            Cell::new(quota.arn().await),
            Cell::new(quota.name().await),
            Cell::new(&quota.utilization().await.unwrap().to_string()),
        ]));
    }

    table.printstd();
}

// use quota::Quota;
// use services::servicequota;
// use std::{collections::HashSet, sync::Arc};
// use tokio::sync::Semaphore;

// #[macro_use]
// extern crate log;

// pub async fn utilization(
//     args: &clap::ArgMatches,
// ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//     let config = config::Config::new(args);
//     log_startup(&config);

//     let mut handlers = Vec::new();

//     for region in config.regions() {
//         info!("checking for quotas in region {}", region);

//         let client = servicequota::Client::new(region).await;
//         let service_codes = client.service_codes().await?;

//         let permits = Arc::new(Semaphore::new(3));

//         for service_code in service_codes {
//             let permits = Arc::clone(&permits);
//             let client_ = client.clone();

//             let handler = tokio::spawn(async move {
//                 utilization_per_service(&client_, &service_code, permits).await
//             });
//             handlers.push(handler)
//         }
//     }

//     let mut all_quotas = Vec::new();
//     for handler in handlers {
//         match handler.await {
//             Ok(Ok(quotas)) => all_quotas.extend(quotas),
//             Ok(Err(e)) => error!("error: {}", e),
//             Err(e) => error!("error while checking quotas: {}", e),
//         }
//     }

//     log_breached_quotas(all_quotas, &config).await;

//     // if let Some(pd_key) = lift_pagerduty_routing_key() {
//     //     let pagerduty = notifiers::pagerduty::Client::new(&pd_key, config.threshold())?;
//     //     notify(pagerduty, &all_quotas, config.ignored_quotas()).await;
//     // }

//     Ok(())
// }

// async fn utilization_per_service(
//     client: &servicequota::Client,
//     service_code: &str,
//     permits: Arc<Semaphore>,
// ) -> Result<Vec<Box<dyn Quota + Sync + Send>>, Box<dyn std::error::Error + Send + Sync>> {
//     let _permits = permits.acquire().await.unwrap();
//     let quotas = client.quotas(service_code).await?;

//     for quota in quotas {
//         quota.utilization().await;
//     }

//     Ok(quotas)
// }

// pub async fn list_quotas(args: &clap::ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
//     let config = config::Config::new(args);

//     for region in config.regions() {
//         let client = servicequota::Client::new(region).await;
//         let service_codes = client.service_codes().await?;

//         for service_code in service_codes {
//             let quotas = client.quotas(&service_code).await?;

//             for quota in quotas {
//                 println!("{:90} {:50}", quota.arn().await, quota.name().await)
//             }
//         }
//     }

//     Ok(())
// }

// fn lift_pagerduty_routing_key() -> Option<String> {
//     std::env::var("PAGERDUTY_ROUTING_KEY").ok()
// }

// // async fn notify(
// //     notifier: impl notifiers::Notifier,
// //     breached_quotas: &Vec<impl Quota>,
// //     ignored_quotas: &HashSet<String>,
// // ) {
// //     for quota in breached_quotas {
// //         if ignored_quotas.contains(&quota.quota_code().to_string()) {
// //             info!(
// //                 "Ignoring quota {} as it is in the ignore list",
// //                 quota.quota_code()
// //             );
// //             continue;
// //         }

// //         let result = notifier.notify(quota).await;

// //         if let Err(err) = result {
// //             println!("pagerduty error: {}", err)
// //         }
// //     }
// // }

// fn log_startup(config: &config::Config) {
//     info!(
//         "Starting up: {} {}",
//         env!("CARGO_PKG_NAME"),
//         env!("CARGO_PKG_VERSION")
//     );
//     info!("Region: {}", config.regions().join(", "));
//     info!("Threshold: {}", config.threshold());
//     info!(
//         "Ignored quotas: {}",
//         config
//             .ignored_quotas()
//             .iter()
//             .map(|s| s.to_string())
//             .collect::<Vec<_>>()
//             .join(", ")
//     );
// }

// async fn log_breached_quotas(quotas: Vec<impl Quota + Sync + Send>, config: &config::Config) {
//     let mut count = 0;

//     for quota in quotas {
//         if quota.utilization().await > Some(config.threshold())
//             && !config.ignored_quotas().contains(quota.quota_code().await)
//         {
//             info!(
//                 "{:15}: {:30} {:12} {:30} : {:3}%",
//                 quota.region().await,
//                 quota.service_code().await,
//                 quota.quota_code().await,
//                 quota.name().await,
//                 quota.utilization().await.unwrap()
//             );

//             count += 1;
//         }
//     }

//     if count == 0 {
//         info!("No quotas breached");
//     }
// }
