use async_trait::async_trait;
use aws_sdk_cloudwatch::types::SdkError;
use aws_sdk_lambda::{error::GetAccountSettingsError, output::GetAccountSettingsOutput};

use crate::{quota::Utilization, util};
use std::{
    error::Error,
    fmt::{Display, Formatter, Result as FmtResult},
};

#[derive(Debug)]
pub enum LambdaError {
    AwsSdkError(SdkError<GetAccountSettingsError>),
}

impl Error for LambdaError {}
impl Display for LambdaError {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            Self::AwsSdkError(e) => write!(f, "AwsSdkError: {}", e),
        }
    }
}

impl From<SdkError<GetAccountSettingsError>> for LambdaError {
    fn from(err: SdkError<GetAccountSettingsError>) -> Self {
        Self::AwsSdkError(err)
    }
}

pub struct Client {
    client: aws_sdk_lambda::Client,
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

    async fn get_account_settings(&self) -> Result<GetAccountSettingsOutput, LambdaError> {
        self.client
            .get_account_settings()
            .send()
            .await
            .map_err(|e| e.into())
    }
}

pub struct LB99A9384 {
    client: Client,
}

impl LB99A9384 {
    pub async fn new(region: &str) -> Self {
        Self {
            client: Client::new(region).await,
        }
    }
}

#[async_trait]
impl Utilization for LB99A9384 {
    type Error = LambdaError;

    async fn utilization(&self) -> Result<u8, Self::Error> {
        let account_settings = self.client.get_account_settings().await?;
        let code_size_limit_bytes = account_settings.account_limit().unwrap().total_code_size();
        let code_size_used_bytes = account_settings.account_usage().unwrap().total_code_size();

        Ok((code_size_used_bytes / code_size_limit_bytes * 100) as u8)
    }
}
