use async_trait::async_trait;
use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::services::cloudwatch::{Client, ServiceQuotaUtilizationQueryInput};

#[derive(Debug)]
pub enum QuotaError {
    ArnFormatError(String),
}

impl Error for QuotaError {}
impl Display for QuotaError {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            Self::ArnFormatError(e) => write!(f, "ArnFormatError: {}", e),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Quota {
    quota_details: QuotaDetails,
    cloudwatch: Option<CloudWatchQuotaDetails>,
    utilization: Arc<RwLock<Option<u8>>>,
}

#[derive(Debug, Clone)]
struct QuotaDetails {
    arn: String,
    account_id: String,
    name: String,
    quota_code: String,
    service_code: String,
    region: String,
}

#[derive(Debug, Clone)]
pub struct CloudWatchQuotaDetails {
    pub client: Client,
    pub query: ServiceQuotaUtilizationQueryInput,
}

#[allow(clippy::redundant_field_names)]
impl Quota {
    pub fn new(
        arn: &str,
        name: &str,
        cloudwatch: Option<CloudWatchQuotaDetails>,
    ) -> Result<Self, QuotaError> {
        let parsed_arn = parse_arn(arn)?;

        Ok(Self {
            quota_details: QuotaDetails {
                arn: arn.to_string(),
                name: name.to_string(),
                account_id: parsed_arn.account_id,
                quota_code: parsed_arn.quota_code,
                service_code: parsed_arn.service_code,
                region: parsed_arn.region,
            },
            cloudwatch: cloudwatch,
            utilization: Arc::new(RwLock::new(None)),
        })
    }

    pub fn name(&self) -> &str {
        &self.quota_details.name
    }

    pub fn arn(&self) -> &str {
        &self.quota_details.arn
    }

    pub fn account_id(&self) -> &str {
        &self.quota_details.account_id
    }

    pub fn quota_code(&self) -> &str {
        &self.quota_details.quota_code
    }

    pub fn service_code(&self) -> &str {
        &self.quota_details.service_code
    }

    pub fn region(&self) -> &str {
        &self.quota_details.region
    }
}

#[async_trait]
pub trait Utilization {
    async fn utilization(&self) -> Option<u8>;
}

#[async_trait]
impl Utilization for Quota {
    async fn utilization(&self) -> Option<u8> {
        if let Some(utilization) = *self.utilization.read().await {
            return Some(utilization);
        }

        if let Some(cloudwatch) = self.cloudwatch.clone() {
            let utilization = cloudwatch
                .client
                .service_quota_utilization(&cloudwatch.query)
                .await
                .ok();

            if let Some(utilization) = utilization {
                *self.utilization.write().await = Some(utilization);
            }

            utilization
        } else {
            None
        }
    }
}

// a ParsedArn defines the individual components of an AWS Service Quota Arn
#[derive(Debug, Clone)]
struct ParsedArn {
    region: String,
    account_id: String,
    service_code: String,
    quota_code: String,
}

// split_arn splits a service quota ARN into the fields of interest, returning:
// region, account, service code, quota code
// An Arn is of the form: arn:${Partition}:servicequotas:${Region}:${Account}:${ServiceCode}/${QuotaCode}
fn parse_arn(arn: &str) -> Result<ParsedArn, QuotaError> {
    // splice the quota code from the end of the Arn
    let (arn_components, quota_code) = arn.split_once('/').ok_or_else(|| {
        QuotaError::ArnFormatError(format!("failed to parse quota code from arn: {}", arn))
    })?;

    // split the remaining components
    let arn_components_vec = arn_components.split(':').collect::<Vec<_>>();

    // evaluate if we have the required length
    if arn_components_vec.len() != 6 {
        return Err(QuotaError::ArnFormatError(format!(
            "failed to parse arn: {}",
            arn
        )));
    }

    Ok(ParsedArn {
        region: arn_components_vec[3].to_string(),
        account_id: arn_components_vec[4].to_string(),
        service_code: arn_components_vec[5].to_string(),
        quota_code: quota_code.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_arn() {
        let arns = vec![(
            "arn:aws:servicequotas:us-east-1:123456789012:service/quota-1",
            ParsedArn {
                region: "us-east-1".to_string(),
                account_id: "123456789012".to_string(),
                service_code: "service".to_string(),
                quota_code: "quota-1".to_string(),
            },
        )];

        for arn in arns {
            let parsed_arn = parse_arn(arn.0).unwrap();

            assert_eq!(parsed_arn.region, arn.1.region);
            assert_eq!(parsed_arn.account_id, arn.1.account_id);
            assert_eq!(parsed_arn.service_code, arn.1.service_code);
            assert_eq!(parsed_arn.quota_code, arn.1.quota_code);
        }
    }

    #[test]
    fn test_parse_arn_errors() {
        let arns = vec![(
            "",
            "arn:aws:servicequotas:us-east-1:123456789012:service",
            "service/quota-1",
        )];

        for arn in arns {
            let parsed_arn = parse_arn(arn.0);

            assert!(parsed_arn.is_err());
        }
    }
}
