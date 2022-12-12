// CloudWatch service APIs for querying quota utilization

use aws_sdk_cloudwatch::error::GetMetricDataError;
use chrono::{Duration, DurationRound, Utc};
use std::collections::HashMap;

use crate::util;
use aws_sdk_cloudwatch::{
    self,
    model::{Dimension, Metric, MetricDataQuery, MetricDataResult, MetricStat},
    types::{DateTime, SdkError},
};
use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};
use tokio_stream::StreamExt;

#[derive(Debug)]
pub enum CloudWatchError {
    MissingMetricData,
    AwsCloudWatchSdkError(SdkError<GetMetricDataError>),
}

impl Error for CloudWatchError {}
impl Display for CloudWatchError {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            Self::MissingMetricData => write!(f, "MissingMetricData"),
            Self::AwsCloudWatchSdkError(e) => write!(f, "AwsCloudWatchSdkError: {}", e),
        }
    }
}

impl From<SdkError<GetMetricDataError>> for CloudWatchError {
    fn from(err: SdkError<GetMetricDataError>) -> Self {
        Self::AwsCloudWatchSdkError(err)
    }
}

#[derive(Debug, Clone)]
pub struct Client {
    client: aws_sdk_cloudwatch::Client,
}

#[derive(Debug, Clone)]
pub struct ServiceQuotaUtilizationQueryInput {
    pub namespace: String,
    pub metric_name: String,
    pub dimensions: HashMap<String, String>,
    pub statistic: String,
}

impl Client {
    pub async fn new(region: &str) -> Self {
        let (config, retries) = util::aws_config_with_region(region).await;
        let client_config = aws_sdk_cloudwatch::config::Builder::from(&config)
            .retry_config(retries)
            .build();
        let client = aws_sdk_cloudwatch::Client::from_conf(client_config);

        Self { client }
    }

    pub async fn service_quota_utilization(
        self,
        query_input: &ServiceQuotaUtilizationQueryInput,
    ) -> Result<u8, CloudWatchError> {
        let (start_time, end_time) = query_times();

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
            .start_time(DateTime::from_secs(start_time as i64))
            .end_time(DateTime::from_secs(end_time as i64))
            .into_paginator()
            .send()
            .collect::<Result<Vec<_>, _>>()
            .await?;

        let mut max_value = None;
        for result in results {
            let r = result.metric_data_results().unwrap();

            if let Some(value) = get_max_value(r) {
                if Some(value) > max_value {
                    max_value = Some(value);
                }
            }
        }

        max_value
            .map(|v| v as u8)
            .ok_or(CloudWatchError::MissingMetricData)
    }
}

// query times are quicker if we are able to sync times to the hour
// https://docs.rs/aws-sdk-cloudwatch/0.21.0/aws_sdk_cloudwatch/struct.Client.html#method.get_metric_data
fn query_times() -> (u64, u64) {
    let end_time = Utc::now()
        .duration_trunc(Duration::hours(1))
        .unwrap()
        .timestamp() as u64;

    let start_time = end_time - 3600;

    (start_time, end_time)
}

fn get_max_value(metric_data_results: &[MetricDataResult]) -> Option<u8> {
    // TODO: max datapoints is 1
    let mut max: Option<u8> = None;

    for metric_data_result in metric_data_results {
        if let Some(values) = metric_data_result.values() {
            for value in values {
                if Some(*value as u8) > max {
                    max = Some(*value as u8)
                }
            }
        }
    }

    max
}

fn hashmap_to_dimensions(hashmap: &HashMap<String, String>) -> Vec<Dimension> {
    hashmap
        .iter()
        .map(|(k, v)| Dimension::builder().name(k).value(v).build())
        .collect::<Vec<_>>()
}
