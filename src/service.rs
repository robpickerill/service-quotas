use aws_sdk_servicequotas;

use crate::{cloudwatch, quota::ServiceQuota, util};
use tokio_stream::StreamExt;

#[derive(Debug, Clone)]
pub struct Client {
    client: aws_sdk_servicequotas::Client,
    cloudwatch_client: cloudwatch::Client,
    threshold: u8,
}

impl Client {
    pub async fn new(threshold: u8) -> Self {
        let client = build_client().await;
        let cloudwatch_client = cloudwatch::Client::new().await;

        Self {
            client: client,
            cloudwatch_client: cloudwatch_client,
            threshold: threshold,
        }
    }

    pub async fn breached_quotas(
        &self,
        service_code: &str,
    ) -> Result<Vec<ServiceQuota>, Box<dyn std::error::Error>> {
        let paginator = self
            .client
            .list_service_quotas()
            .service_code(service_code)
            .into_paginator()
            .items()
            .send();

        println!(
            "calculating utilization for quotas in service: {}",
            service_code
        );

        let quotas = paginator.collect::<Result<Vec<_>, _>>().await?;

        let mut breached_quotas: Vec<ServiceQuota> = Vec::new();
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

                let utilization = cw.service_quota_utilization(&query_input).await.ok();

                // result.push(ServiceQuota::new(
                //     quota.quota_name().unwrap(),
                //     quota.service_code().unwrap(),
                //     utilization,
                // ));

                if utilization > Some(self.threshold) {
                    breached_quotas.push(ServiceQuota::new(
                        quota.quota_name().unwrap(),
                        quota.service_code().unwrap(),
                        utilization,
                    ))
                };
            }
        }

        return Ok(breached_quotas);
    }
}

async fn build_client() -> aws_sdk_servicequotas::Client {
    let (config, retries) = util::aws_config().await;
    let client_config = aws_sdk_servicequotas::config::Builder::from(&config)
        .retry_config(retries)
        .build();
    aws_sdk_servicequotas::Client::from_conf(client_config)
}

pub async fn list_service_codes() -> Vec<String> {
    let client = build_client().await;
    let result = client
        .list_services()
        .into_paginator()
        .items()
        .send()
        .collect::<Result<Vec<_>, _>>()
        .await
        .unwrap();

    result
        .into_iter()
        .map(|s| s.service_code().unwrap().to_string())
        .collect::<Vec<_>>()
}
