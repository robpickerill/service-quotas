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
    let mut all_quotas = Vec::new();
    for region in regions {
        let client = servicequota::Client::new(region).await;
        let service_codes = client.service_codes().await?;

        for service_code in service_codes {
            all_quotas.extend(client.quotas(&service_code).await?);
        }
    }

    print_list_quotas_table(all_quotas).await;

    Ok(())
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

    table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
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

        let permits = Arc::new(Semaphore::new(4));

        for service_code in service_codes {
            let permits = Arc::clone(&permits);
            let client_ = client.clone();

            let handler = tokio::spawn(async move {
                utilization_per_service(&client_, &service_code, permits).await
            });
            handlers.push(handler)
        }
    }

    let mut all_quotas = Vec::new();
    for handler in handlers {
        let result = handler.await?;

        match result {
            Ok(quotas) => all_quotas.extend(quotas),
            Err(err) => println!("error: {}", err),
        }
    }

    print_breached_quotas_table(&all_quotas, threshold).await;
    notify_breached_quotas(&all_quotas, threshold, ignored_quotas.as_deref()).await?;

    Ok(())
}

async fn utilization_per_service(
    client: &servicequota::Client,
    service_code: &str,
    permits: Arc<Semaphore>,
) -> Result<Vec<Box<dyn Quota>>, Box<dyn Error + Sync + Send>> {
    let _permits = permits.acquire().await.unwrap();
    let quotas = client.quotas(service_code).await;

    match quotas {
        Ok(quotas) => {
            // TODO: This is a hack to get utilization in parallel
            for quota in &quotas {
                quota.utilization().await;
            }

            Ok(quotas)
        }
        Err(err) => Err(Box::new(err)),
    }
}

async fn print_breached_quotas_table(quotas: &[Box<dyn Quota>], threshold: &u8) {
    let mut table = Table::new();
    table.add_row(row!["ARN", "Quota Name", "Utilization"]);

    for quota in quotas {
        if quota.utilization().await > Some(*threshold) {
            table.add_row(Row::new(vec![
                Cell::new(quota.arn().await),
                Cell::new(quota.name().await),
                Cell::new(&quota.utilization().await.unwrap().to_string()),
            ]));
        }
    }

    table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
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
