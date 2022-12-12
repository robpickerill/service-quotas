pub mod cli;

mod quota;
mod services;
mod util;

#[macro_use]
extern crate prettytable;

use clap::ArgMatches;
use prettytable::{format, Cell, Row, Table};
use quota::Quota;
use services::servicequota;
use std::sync::Arc;
use tokio::sync::Semaphore;

pub async fn list_quotas(args: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: Move this complexity into the servicequota module
    let regions = args.get_many::<String>("regions").unwrap();

    let tasks = regions
        .map(|r| async {
            let client = servicequota::Client::new(r).await;
            let service_codes = client.service_codes().await.unwrap();

            let permits = Arc::new(Semaphore::new(2));

            service_codes
                .into_iter()
                .map(|s| {
                    let client_ = client.clone();
                    let permits = Arc::clone(&permits);
                    tokio::spawn(async move {
                        let _permits = permits.acquire().await.unwrap();
                        match client_.quotas(&s).await {
                            Ok(quotas) => Ok(quotas),
                            Err(err) => Err(err),
                        }
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let mut all_quotas = Vec::new();
    for task in tasks {
        for quotas in task.await {
            match quotas.await {
                Ok(quotas_) => match quotas_ {
                    Ok(quotas_) => all_quotas.extend(quotas_),
                    Err(err) => println!("error: {}", err),
                },
                Err(err) => println!("error: {}", err),
            }
        }
    }

    print_list_quotas_table(all_quotas).await;

    Ok(())
}

async fn print_list_quotas_table(quotas: Vec<Box<dyn Quota>>) {
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
    table.set_titles(Row::new(vec![Cell::new("Arn"), Cell::new("Name")]));

    for quota in quotas {
        table.add_row(Row::new(vec![
            Cell::new(quota.arn().await),
            Cell::new(quota.name().await),
        ]));
    }

    table.printstd();
}

pub async fn utilization(args: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: Move this complexity into the servicequota module
    let regions = args.get_many::<String>("regions").unwrap();
    let threshold = args.get_one::<u8>("threshold").unwrap();

    let tasks = regions
        .map(|r| async {
            let client = servicequota::Client::new(r).await;
            let service_codes = client.service_codes().await.unwrap();

            let permits = Arc::new(Semaphore::new(2));

            service_codes
                .into_iter()
                .map(|s| {
                    let client_ = client.clone();
                    let threshold_ = *threshold;
                    let permits = Arc::clone(&permits);
                    tokio::spawn(async move {
                        let _permits = permits.acquire().await.unwrap();
                        match client_.quotas(&s).await {
                            Ok(quotas) => {
                                let mut breached = vec![];
                                for quota in quotas {
                                    if quota.utilization().await > Some(threshold_) {
                                        breached.push(quota);
                                    }
                                }
                                Ok(breached)
                            }
                            Err(err) => Err(err),
                        }
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let mut breached_quotas = Vec::new();
    for task in tasks {
        for quotas in task.await {
            match quotas.await {
                Ok(quotas_) => match quotas_ {
                    Ok(quotas_) => {
                        for q in quotas_ {
                            breached_quotas.push(q);
                        }
                    }

                    Err(err) => println!("error: {}", err),
                },
                Err(err) => println!("error: {}", err),
            }
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
