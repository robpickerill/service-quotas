use crate::{quota::Quota, services::cloudwatch, util};
use tokio_stream::StreamExt;

#[derive(Debug, Clone)]
pub struct Client {
    client: aws_sdk_servicequotas::Client,
    cloudwatch_client: cloudwatch::Client,
    region: String,
    threshold: u8,
}

impl Client {
    pub async fn new(region: &str, threshold: u8) -> Self {
        let client = build_client(region).await;
        let cloudwatch_client = cloudwatch::Client::new(region).await;

        Self {
            client,
            cloudwatch_client,
            region: region.to_string(),
            threshold,
        }
    }

    pub async fn breached_quotas(
        &self,
        service_code: &str,
    ) -> Result<Vec<Quota>, Box<dyn std::error::Error>> {
        let paginator = self
            .client
            .list_service_quotas()
            .service_code(service_code)
            .into_paginator()
            .items()
            .send();

        info!(
            "calculating utilization for quotas in region: {} for service: {}",
            self.region, service_code
        );

        let quotas = paginator.collect::<Result<Vec<_>, _>>().await?;

        let mut breached_quotas: Vec<Quota> = Vec::new();
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

                if utilization > Some(self.threshold) {
                    breached_quotas.push(Quota::new(
                        quota.quota_name().unwrap(),
                        quota.service_code().unwrap(),
                        quota.quota_code().unwrap(),
                        &self.region,
                        utilization,
                    ))
                };
            }
        }

        Ok(breached_quotas)
    }
}

async fn build_client(region: &str) -> aws_sdk_servicequotas::Client {
    let (config, retries) = util::aws_config_with_region(region).await;
    let client_config = aws_sdk_servicequotas::config::Builder::from(&config)
        .retry_config(retries)
        .build();
    aws_sdk_servicequotas::Client::from_conf(client_config)
}

pub async fn list_service_codes(region: &str) -> Vec<String> {
    let client = build_client(region).await;
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
