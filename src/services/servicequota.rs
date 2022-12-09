use crate::quota::CloudWatchQuotaDetails;
use crate::util;
use crate::{quota::Quota, quota::QuotaError, services::cloudwatch};

use aws_sdk_cloudwatch::types::SdkError;
use aws_sdk_servicequotas::error::{ListServiceQuotasError, ListServicesError};
use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};
use tokio_stream::StreamExt;

#[derive(Debug)]
pub enum ServiceQuotaError {
    QuotaError(QuotaError),
    AwsSdkErrorListServiceQuotas(SdkError<ListServiceQuotasError>),
    AwsSdkErrorListServices(SdkError<ListServicesError>),
}

impl Error for ServiceQuotaError {}
impl Display for ServiceQuotaError {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            Self::QuotaError(e) => write!(f, "QuotaError: {}", e),
            Self::AwsSdkErrorListServiceQuotas(e) => write!(f, "AwsSdkError: {}", e),
            Self::AwsSdkErrorListServices(e) => write!(f, "AwsSdkError: {}", e),
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
        Self::AwsSdkErrorListServiceQuotas(err)
    }
}
impl From<SdkError<ListServicesError>> for ServiceQuotaError {
    fn from(err: SdkError<ListServicesError>) -> Self {
        Self::AwsSdkErrorListServices(err)
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

    pub async fn quotas(&self, service_code: &str) -> Result<Vec<Quota>, ServiceQuotaError> {
        let paginator = self
            .client
            .list_service_quotas()
            .service_code(service_code)
            .into_paginator()
            .items()
            .send();

        let all_quotas = paginator.collect::<Result<Vec<_>, _>>().await?;

        let mut quotas: Vec<Quota> = Vec::new();
        for quota in all_quotas {
            let cw = self.cloudwatch_client.clone();

            // TODO: handle all the unwraps here
            match quota.usage_metric() {
                Some(metric_info) => {
                    let query_input = cloudwatch::ServiceQuotaUtilizationQueryInput {
                        namespace: metric_info.metric_namespace().unwrap().to_string(),
                        metric_name: metric_info.metric_name().unwrap().to_string(),
                        dimensions: metric_info.metric_dimensions().unwrap().clone(),
                        statistic: metric_info
                            .metric_statistic_recommendation()
                            .unwrap()
                            .to_string(),
                    };

                    quotas.push(Quota::new(
                        quota.quota_arn().unwrap(),
                        quota.quota_name().unwrap(),
                        Some(CloudWatchQuotaDetails {
                            client: cw,
                            query: query_input,
                        }),
                    )?);
                }
                None => {
                    quotas.push(Quota::new(
                        quota.quota_arn().unwrap(),
                        quota.quota_name().unwrap(),
                        None,
                    )?);
                }
            }
        }

        Ok(quotas)
    }
}
