use crate::{
    quota,
    quota::{Quota, QuotaError},
    util,
};
use async_trait::async_trait;
use aws_sdk_lambda::{
    self, error::GetAccountSettingsError, output::GetAccountSettingsOutput, types::SdkError,
};
use std::{
    error::Error,
    fmt::{Display, Formatter, Result as FmtResult},
    sync::Arc,
};
use tokio::sync::RwLock;

struct Client {
    client: aws_sdk_lambda::Client,
}

#[derive(Debug)]
pub enum LambdaError {
    AwsSdkError(SdkError<GetAccountSettingsError>),
    // issues with parsing ARNs
    ArnFormatError(String),
}

impl Error for LambdaError {}
impl Display for LambdaError {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            Self::AwsSdkError(e) => write!(f, "AwsSdkError: {}", e),
            Self::ArnFormatError(e) => write!(f, "ArnFormatError: {}", e),
        }
    }
}

impl From<SdkError<GetAccountSettingsError>> for LambdaError {
    fn from(err: SdkError<GetAccountSettingsError>) -> Self {
        Self::AwsSdkError(err)
    }
}

impl From<QuotaError> for LambdaError {
    fn from(err: QuotaError) -> Self {
        Self::ArnFormatError(err.to_string())
    }
}

impl Client {
    pub async fn new(region: &str) -> Self {
        let (config, retries) = util::aws_config_with_region(region).await;

        let client_config = aws_sdk_lambda::config::Builder::from(&config)
            .retry_config(retries)
            .build();

        let client = aws_sdk_lambda::Client::from_conf(client_config);

        Self { client }
    }

    pub async fn get_account_settings(&self) -> Result<GetAccountSettingsOutput, LambdaError> {
        self.client
            .get_account_settings()
            .send()
            .await
            .map_err(|e| e.into())
    }
}

// The amount of storage that's available for deployment packages and layer archives in the current Region.
pub struct QuotaL2ACBD22F {
    client: Client,
    utilization: Arc<RwLock<Option<u8>>>,
    arn: String,
    account_id: String,
    name: String,
    quota_code: String,
    service_code: String,
    region: String,
}

#[allow(clippy::redundant_field_names)]
impl QuotaL2ACBD22F {
    pub async fn new(arn: &str, name: &str) -> Result<Self, LambdaError> {
        let parsed_arn = quota::parse_arn(arn)?;
        let client = Client::new(&parsed_arn.region).await;

        Ok(Self {
            client: client,
            utilization: Arc::new(RwLock::new(None)),
            arn: arn.to_string(),
            name: name.to_string(),
            account_id: parsed_arn.account_id,
            quota_code: parsed_arn.quota_code,
            service_code: parsed_arn.service_code,
            region: parsed_arn.region,
        })
    }
}

#[async_trait]
impl Quota for QuotaL2ACBD22F {
    async fn arn(&self) -> &str {
        &self.arn
    }

    async fn account_id(&self) -> &str {
        &self.account_id
    }

    async fn name(&self) -> &str {
        &self.name
    }

    async fn quota_code(&self) -> &str {
        &self.quota_code
    }

    async fn service_code(&self) -> &str {
        &self.service_code
    }

    async fn region(&self) -> &str {
        &self.region
    }

    async fn utilization(&self) -> Option<u8> {
        if let Some(utilization) = *self.utilization.read().await {
            return Some(utilization);
        }

        let response = self.client.get_account_settings().await.ok()?;

        if let (Some(account_usage), Some(account_limit)) =
            (response.account_usage(), response.account_limit())
        {
            let used = account_usage.total_code_size();
            let limit = account_limit.total_code_size();

            let utilization = (used / limit * 100) as u8;

            *self.utilization.write().await = Some(utilization);

            return Some(utilization);
        }

        None
    }
}
