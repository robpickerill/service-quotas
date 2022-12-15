pub mod cli;

mod notifiers;
mod quotas;
mod services;
mod util;

#[macro_use]
extern crate prettytable;

use clap::ArgMatches;
use notifiers::Notify;
use prettytable::{format, Cell, Row, Table};
use quotas::Quota;
use services::servicequota;
use std::{error::Error, sync::Arc};
use tokio::sync::Semaphore;

pub async fn list_quotas(args: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let regions = args.get_many::<String>("regions").unwrap();

    // TODO: Move this complexity into the servicequota module
    let mut handlers = Vec::new();
    for region in regions {
        println!("checking for quotas in region {}", region);

        let client = servicequota::Client::new(region).await;
        let service_codes = client.service_codes().await?;

        let permits = new_permits().await;

        for service_code in service_codes {
            let client_ = client.clone();
            let permits = Arc::clone(&permits);

            handlers.push(tokio::spawn(async move {
                let _permits = permits.acquire().await.unwrap();
                client_.quotas(&service_code).await
            }));
        }
    }

    let mut all_quotas = Vec::new();
    for handler in handlers {
        match handler.await {
            Ok(result) => match result {
                Ok(quotas) => all_quotas.extend(quotas),
                Err(err) => println!("error: {}", err),
            },
            Err(err) => println!("error: {}", err),
        };
    }

    print_list_quotas_table(all_quotas).await;

    Ok(())
}

// new_permits returns a new semaphore.
// 3 concurrent requests to the AWS APIs feels like a good number to avoid getting
// rate limited.
// TODO: Make this configurable
async fn new_permits() -> Arc<Semaphore> {
    Arc::new(Semaphore::new(3))
}

async fn print_list_quotas_table(quotas: Vec<Box<dyn Quota>>) {
    let mut table = Table::new();
    table.set_titles(Row::new(vec![Cell::new("Arn"), Cell::new("Name")]));

    for quota in quotas {
        table.add_row(Row::new(vec![
            Cell::new(quota.arn().await),
            Cell::new(quota.name().await),
        ]));
    }

    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    table.printstd();
}

pub async fn utilization(args: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let regions = args.get_many::<String>("regions").unwrap();
    let threshold = args.get_one::<u8>("threshold").unwrap();
    let ignored_quotas = match args.try_get_many::<String>("ignore") {
        Ok(Some(ignored_quotas)) => Some(ignored_quotas.map(|r| r.to_string()).collect::<Vec<_>>()),
        _ => None,
    };

    let mut handlers = Vec::new();

    // TODO: Move this complexity into the servicequota module
    for region in regions {
        println!("checking for quotas in region {}", region);

        let client = servicequota::Client::new(region).await;
        let service_codes = client.service_codes().await?;

        let permits = new_permits().await;

        for service_code in service_codes {
            let permits = Arc::clone(&permits);
            let client_ = client.clone();
            let threshold_ = *threshold;

            let handler = tokio::spawn(async move {
                utilization_per_service(&client_, &service_code, &threshold_, permits).await
            });
            handlers.push(handler)
        }
    }

    let mut all_quotas = Vec::new();
    let mut breached_quotas = Vec::new();
    for handler in handlers {
        let result = handler.await?;

        match result {
            Ok(quotas) => {
                all_quotas.extend(quotas.0);
                breached_quotas.extend(quotas.1);
            }
            Err(err) => println!("error: {}", err),
        }
    }

    print_breached_quotas_table(&breached_quotas).await;
    notify_breached_quotas(&breached_quotas, threshold, ignored_quotas.as_deref()).await?;

    Ok(())
}

async fn utilization_per_service(
    client: &servicequota::Client,
    service_code: &str,
    threshold: &u8,
    permits: Arc<Semaphore>,
) -> Result<(Vec<Box<dyn Quota>>, Vec<Box<dyn Quota>>), Box<dyn Error + Sync + Send>> {
    let _permits = permits.acquire().await.unwrap();
    let quotas = client.quotas(service_code).await;

    match quotas {
        Ok(quotas) => {
            let mut breached_quotas = Vec::new();
            for quota in &quotas {
                if quota.utilization().await > Some(*threshold) {
                    breached_quotas.push(quota.clone());
                }
            }
            Ok((quotas, breached_quotas))
        }
        Err(err) => Err(Box::new(err)),
    }
}

async fn print_breached_quotas_table(quotas: &[Box<dyn Quota>]) {
    let mut table = Table::new();
    table.add_row(row!["ARN", "Quota Name", "Utilization"]);

    for quota in quotas {
        table.add_row(Row::new(vec![
            Cell::new(quota.arn().await),
            Cell::new(quota.name().await),
            Cell::new(&quota.utilization().await.unwrap().to_string()),
        ]));
    }

    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    table.printstd();
}

async fn notify_breached_quotas(
    quotas: &[Box<dyn Quota>],
    threshold: &u8,
    ignored_quotas: Option<&[String]>,
) -> Result<(), Box<dyn std::error::Error>> {
    // if we found a notifier, then use it to send notifications
    if let Some(notifier) = notifiers::lookup_notifiers(threshold, ignored_quotas).await? {
        notifier.notify(quotas).await?;
    };

    Ok(())
}
