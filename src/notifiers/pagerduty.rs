use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, InvalidHeaderValue, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use std::convert::From;
use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};

use crate::notifiers::Notifier;
use crate::quota::Quota;

#[derive(Debug)]
pub enum ClientError {
    ReqwestError(reqwest::Error),
    InvalidHeaderValue(InvalidHeaderValue),

    // https://developer.pagerduty.com/docs/ZG9jOjExMDI5NTgw-events-api-v2-overview#response-codes--retry-logic
    PagerdutyApiError(u16, String),
}

impl Error for ClientError {}
impl Display for ClientError {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            Self::ReqwestError(e) => write!(f, "RequestError: {}", e),
            Self::InvalidHeaderValue(e) => write!(f, "InvalidHeaderValue: {}", e),
            Self::PagerdutyApiError(status_code, error) => {
                write!(
                    f,
                    "Pagerduty API Error statuscode: {}, error: {}, ",
                    status_code, error
                )
            }
        }
    }
}

impl From<reqwest::Error> for ClientError {
    fn from(err: reqwest::Error) -> Self {
        Self::ReqwestError(err)
    }
}

impl From<InvalidHeaderValue> for ClientError {
    fn from(err: InvalidHeaderValue) -> Self {
        Self::InvalidHeaderValue(err)
    }
}

pub struct Client {
    client: reqwest::Client,
    routing_key: String,
    threshold: u8,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NotifyBody {
    routing_key: String,
    event_action: String,
    dedup_key: String,
    payload: Payload,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Payload {
    summary: String,
    source: String,
    severity: String,
    custom_details: CustomDetails,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CustomDetails {
    arn: String,
    account_id: String,
    service: String,
    region: String,
    quota_name: String,
    quota_code: String,
    utilization_percentage: u8,
    threshold: u8,
    service_quota_url: String,
}

impl Client {
    pub fn new(routing_key: &str, threshold: u8) -> Result<Client, ClientError> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_str("application/json")?);

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(Self {
            client,
            threshold,
            routing_key: routing_key.to_string(),
        })
    }

    fn dedup_key(&self, quota: &Quota) -> String {
        format!(
            "{}-{}-{}",
            quota.quota_code(),
            quota.region(),
            quota.account_id()
        )
    }

    fn trigger_action(&self, utilization: Option<u8>) -> String {
        if utilization >= Some(self.threshold) {
            return String::from("trigger");
        }

        String::from("resolve")
    }
}

#[async_trait]
impl Notifier for Client {
    type Error = ClientError;

    #[allow(clippy::redundant_field_names)]
    async fn notify(&self, quota: Quota) -> Result<(), Self::Error> {
        let url = "https://events.pagerduty.com/v2/enqueue";
        let trigger_action = self.trigger_action(quota.utilization());
        let dedup_key = self.dedup_key(&quota);

        let Some(utilization) = quota.utilization() else {
            return Ok(());
        };

        let payload = NotifyBody {
            routing_key: self.routing_key.clone(),
            event_action: trigger_action,
            dedup_key: dedup_key,
            payload: Payload {
                summary: format!(
                    "Service Quota Utilization {}%: {} - {} in {} - {}",
                    utilization,
                    quota.quota_code(),
                    quota.name(),
                    quota.account_id(),
                    quota.region(),
                ),
                source: "https://github.com/robpickerill/service-quotas".to_string(),
                severity: "warning".to_string(),
                custom_details: CustomDetails {
                    arn: quota.arn().to_string(),
                    account_id: quota.account_id().to_string(),
                    service: quota.service_code().to_string(),
                    region: quota.region().to_string(),
                    quota_name: quota.name().to_string(),
                    quota_code: quota.quota_code().to_string(),
                    threshold: self.threshold,
                    utilization_percentage: utilization,
                    service_quota_url: service_quota_url(&quota),
                },
            },
        };

        let result = self.client.post(url).json(&payload).send().await?;

        match result.status().as_u16() {
            202 => Ok(()),
            _ => Err(ClientError::PagerdutyApiError(
                result.status().as_u16(),
                result.text().await?,
            )),
        }
    }
}

// The url format for the a service quota in the AWS console
// example: https://us-east-1.console.aws.amazon.com/servicequotas/home/services/ec2/quotas/L-85EED4F7
fn service_quota_url(quota: &Quota) -> String {
    format!(
        "https://{}.console.aws.amazon.com/servicequotas/home/services/{}/quotas/{}",
        quota.region(),
        quota.service_code(),
        quota.quota_code()
    )
}
