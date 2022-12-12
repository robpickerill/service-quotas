use crate::quotas::{
    lambda::QuotaL2ACBD22F, CloudWatchQuotaDetails, Quota, QuotaCloudWatch, QuotaError,
};
use crate::services::cloudwatch;
use crate::util;

use aws_sdk_cloudwatch::types::SdkError;
use aws_sdk_servicequotas::error::{ListServiceQuotasError, ListServicesError};
use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};
use tokio_stream::StreamExt;

#[derive(Debug)]
pub enum ServiceQuotaError {
    QuotaError(QuotaError),
    AwsServiceQuotasSdkErrorListServiceQuotas(SdkError<ListServiceQuotasError>),
    AwsServiceQuotasSdkErrorListServices(SdkError<ListServicesError>),
}

impl Error for ServiceQuotaError {}
impl Display for ServiceQuotaError {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            Self::QuotaError(e) => write!(f, "QuotaError: {}", e),
            Self::AwsServiceQuotasSdkErrorListServiceQuotas(e) => {
                write!(f, "AwsServiceQuotasSdkErrorListServiceQuotas: {}", e)
            }
            Self::AwsServiceQuotasSdkErrorListServices(e) => {
                write!(f, "AwsServiceQuotasSdkErrorListServices: {}", e)
            }
        }
    }
}

impl From<QuotaError> for ServiceQuotaError {
    fn from(err: QuotaError) -> Self {
        Self::QuotaError(err)
    }
}
impl From<SdkError<ListServiceQuotasError>> for ServiceQuotaError {
    fn from(err: SdkError<ListServiceQuotasError>) -> Self {
        Self::AwsServiceQuotasSdkErrorListServiceQuotas(err)
    }
}
impl From<SdkError<ListServicesError>> for ServiceQuotaError {
    fn from(err: SdkError<ListServicesError>) -> Self {
        Self::AwsServiceQuotasSdkErrorListServices(err)
    }
}

#[derive(Debug, Clone)]
pub struct Client {
    client: aws_sdk_servicequotas::Client,
    cloudwatch_client: cloudwatch::Client,
}

impl Client {
    pub async fn new(region: &str) -> Self {
        let (config, retries) = util::aws_config_with_region(region).await;
        let client_config = aws_sdk_servicequotas::config::Builder::from(&config)
            .retry_config(retries)
            .build();
        let client = aws_sdk_servicequotas::Client::from_conf(client_config);

        let cloudwatch_client = cloudwatch::Client::new(region).await;

        Self {
            client,
            cloudwatch_client,
        }
    }

    pub async fn service_codes(&self) -> Result<Vec<String>, ServiceQuotaError> {
        let result = self
            .client
            .list_services()
            .into_paginator()
            .items()
            .send()
            .collect::<Result<Vec<_>, _>>()
            .await?;

        Ok(result
            .into_iter()
            .map(|s| s.service_code().unwrap().to_string())
            .collect::<Vec<_>>())
    }

    pub async fn quotas(
        &self,
        service_code: &str,
    ) -> Result<Vec<Box<dyn Quota>>, ServiceQuotaError> {
        let paginator = self
            .client
            .list_service_quotas()
            .service_code(service_code)
            .into_paginator()
            .items()
            .send();

        let all_quotas = paginator.collect::<Result<Vec<_>, _>>().await?;

        let mut quotas: Vec<Box<dyn Quota>> = Vec::new();
        for quota in all_quotas {
            let cw = self.cloudwatch_client.clone();

            if let Some(usage_metric) = quota.usage_metric() {
                let query_input = cloudwatch::ServiceQuotaUtilizationQueryInput {
                    namespace: usage_metric.metric_namespace().unwrap().to_string(),
                    metric_name: usage_metric.metric_name().unwrap().to_string(),
                    dimensions: usage_metric.metric_dimensions().unwrap().clone(),
                    statistic: usage_metric
                        .metric_statistic_recommendation()
                        .unwrap()
                        .to_string(),
                };

                let new_quota = QuotaCloudWatch::new(
                    quota.quota_arn().unwrap(),
                    quota.quota_name().unwrap(),
                    Some(CloudWatchQuotaDetails {
                        client: cw,
                        query: query_input,
                    }),
                )?;

                quotas.push(Box::new(new_quota));
                continue;
            } else {
                let quota_code = quota.quota_code().unwrap();
                let arn = quota.quota_arn().unwrap();
                let name = quota.quota_name().unwrap();

                if let Some(quota_result) = lookup_quota(quota_code, arn, name).await {
                    quotas.push(quota_result);
                }
            }
        }

        Ok(quotas)
    }
}

// lookup_quota provides a lookup table for Quotas that are not supported by the CloudWatch API,
// i.e. manually implemented quotas.
async fn lookup_quota(quota_code: &str, arn: &str, name: &str) -> Option<Box<dyn Quota>> {
    match quota_code {
        "L-2ACBD22F" => Some(Box::new(QuotaL2ACBD22F::new(arn, name).await.unwrap())),
        _ => None,
    }
}
