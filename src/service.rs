use aws_sdk_servicequotas;

use crate::{cloudwatch, util};
use tokio_stream::StreamExt;

#[derive(Debug, Clone)]
pub struct Client {
    client: aws_sdk_servicequotas::Client,
    cloudwatch_client: cloudwatch::Client,
}

impl Client {
    pub async fn new() -> Self {
        let (config, retries) = util::aws_config().await;
        let client_config = aws_sdk_servicequotas::config::Builder::from(&config)
            .retry_config(retries)
            .build();
        let client = aws_sdk_servicequotas::Client::from_conf(client_config);

        let cloudwatch_client = cloudwatch::Client::new().await;

        Self {
            client,
            cloudwatch_client,
        }
    }

    pub async fn quotas(&self, service_code: &str) -> Result<String, Box<dyn std::error::Error>> {
        let paginator = self
            .client
            .list_service_quotas()
            .service_code(service_code)
            .into_paginator()
            .items()
            .send();

        let quotas = paginator.collect::<Result<Vec<_>, _>>().await?;

        for quota in quotas {
            let cw = self.cloudwatch_client.clone();

            if let Some(metric_info) = quota.usage_metric() {
                let query_input = cloudwatch::ServiceQuotaUtilizationQueryInput {
                    namespace: metric_info.metric_namespace().unwrap().to_string(),
                    metric_name: metric_info.metric_name().unwrap().to_string(),
                    dimensions: metric_info.metric_dimensions().unwrap().clone(),
                    statistic: metric_info
                        .metric_statistic_recommendation()
                        .unwrap()
                        .to_string(),
                };

                cw.service_quota_utilization(&query_input).await?;
            }
        }

        return Ok("a".to_string());
    }
}
