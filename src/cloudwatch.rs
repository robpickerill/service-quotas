use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::util;
use aws_sdk_cloudwatch::{
    self,
    model::{Dimension, Metric, MetricDataQuery, MetricStat},
    types::DateTime,
};
use tokio_stream::StreamExt;

#[derive(Debug, Clone)]
pub struct Client {
    client: aws_sdk_cloudwatch::Client,
}

pub struct ServiceQuotaUtilizationQueryInput {
    pub namespace: String,
    pub metric_name: String,
    pub dimensions: HashMap<String, String>,
    pub statistic: String,
}

impl Client {
    pub async fn new() -> Self {
        let (config, retries) = util::aws_config().await;
        let client_config = aws_sdk_cloudwatch::config::Builder::from(&config)
            .retry_config(retries)
            .build();
        let client = aws_sdk_cloudwatch::Client::from_conf(client_config);

        Self { client }
    }

    pub async fn service_quota_utilization(
        self,
        query_input: &ServiceQuotaUtilizationQueryInput,
    ) -> Result<u8, Box<dyn std::error::Error>> {
        let end_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let start_time = end_time - 600;

        let dimensions = hashmap_to_dimensions(&query_input.dimensions);

        let metric = Metric::builder()
            .set_dimensions(Some(dimensions))
            .metric_name(&query_input.metric_name)
            .namespace(&query_input.namespace)
            .build();

        let metric_stat = MetricStat::builder()
            .metric(metric)
            .period(60)
            .stat(&query_input.statistic)
            .build();

        let usage_data = MetricDataQuery::builder()
            .metric_stat(metric_stat)
            .id("usage_data")
            .return_data(false)
            .build();

        let percentage_usage_data = MetricDataQuery::builder()
            .expression("(usage_data/SERVICE_QUOTA(usage_data))*100")
            .id("utilization")
            .return_data(true)
            .build();

        let results = self
            .client
            .get_metric_data()
            .metric_data_queries(usage_data)
            .metric_data_queries(percentage_usage_data)
            .max_datapoints(10)
            .start_time(DateTime::from_secs(start_time as i64))
            .end_time(DateTime::from_secs(end_time as i64))
            .into_paginator()
            .send()
            .collect::<Result<Vec<_>, _>>()
            .await?;

        for result in results {
            println!("{:?}", result.metric_data_results());
        }
        Ok(0)
    }
}

fn hashmap_to_dimensions(hashmap: &HashMap<String, String>) -> Vec<Dimension> {
    hashmap
        .into_iter()
        .map(|(k, v)| Dimension::builder().name(k).value(v).build())
        .collect::<Vec<_>>()
}
