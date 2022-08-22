use async_trait::async_trait;
use std::collections::HashMap;

use aws_sdk_cloudwatch::model::{metric_datum, Dimension, Metric, MetricDataQuery, MetricStat};
use aws_sdk_servicequotas::{
    self,
    model::{ServiceInfo, ServiceQuota},
    RetryConfig,
};
use tokio_stream::StreamExt;

#[async_trait]
trait Quota {
    async fn cloudwatch_client(&self) -> &aws_sdk_cloudwatch::Client;

    async fn utilization(&self, quota: &ServiceQuota) -> Option<f64> {
        if quota.usage_metric().is_none() {
            return None;
        }

        let usage_metric = quota.usage_metric().unwrap();

        let namespace = usage_metric.metric_namespace().unwrap();
        let metric_name = usage_metric.metric_name().unwrap();
        let dimension = dimension(usage_metric.metric_dimensions().unwrap())?;
        let statistic = usage_metric.metric_statistic_recommendation().unwrap();

        let m1 = Metric::builder()
            .namespace(namespace)
            .dimensions(dimension)
            .metric_name(metric_name)
            .build();

        let metric_stat = MetricStat::builder()
            .metric(m1)
            .period(60)
            .stat(statistic)
            .build();

        let metric_data_query = MetricDataQuery::builder()
            .id("m1")
            .metric_stat(metric_stat)
            .period(60)
            .build();

        let result = self
            .cloudwatch_client()
            .await
            .get_metric_data()
            .metric_data_queries(metric_data_query)
            .send()
            .await;

        let metric_result = result
            .unwrap()
            .metric_data_results()
            .unwrap()
            .iter()
            .last()
            .map(|e| e.values().unwrap())
            .unwrap()
            .last();

        Some(metric_result)
    }
}

fn dimension(dimension: &HashMap<String, String>) -> Option<Dimension> {
    if let (Some(name), Some(value)) = (dimension.get("name"), dimension.get("value")) {
        Some(Dimension::builder().name(name).value(value).build())
    } else {
        None
    }
}

#[derive(Clone, Debug)]
pub struct Client {
    client: aws_sdk_servicequotas::Client,
}

impl Client {
    pub async fn new() -> Self {
        let config = aws_config::from_env()
            .retry_config(RetryConfig::new().with_max_attempts(5))
            .load()
            .await;
        Self {
            client: aws_sdk_servicequotas::Client::new(&config),
        }
    }

    pub async fn get_quotas(
        &self,
    ) -> Result<Vec<aws_sdk_servicequotas::model::ServiceQuota>, aws_sdk_servicequotas::Error> {
        let services = self.clone().list_services().await?;

        let mut results: Vec<ServiceQuota> = Vec::new();
        for service in services {
            let service_code = service.service_code().unwrap();
            let result = self.clone().list_service_quotas(service_code).await?;
            for r in result {
                results.push(r)
            }
        }

        Ok(results)
    }

    async fn list_services(self) -> Result<Vec<ServiceInfo>, aws_sdk_servicequotas::Error> {
        let paginator = self.client.list_services().into_paginator().items().send();
        paginator
            .collect::<Result<Vec<_>, _>>()
            .await
            .map_err(|e| e.into())
    }

    async fn list_service_quotas(
        self,
        service_code: &str,
    ) -> Result<Vec<aws_sdk_servicequotas::model::ServiceQuota>, aws_sdk_servicequotas::Error> {
        let paginator = self
            .client
            .list_service_quotas()
            .service_code(service_code)
            .into_paginator()
            .items()
            .send();
        paginator
            .collect::<Result<Vec<_>, _>>()
            .await
            .map_err(|e| e.into())
    }
}
