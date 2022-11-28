use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};

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
    arn: String,
    account_id: String,
    name: String,
    quota_code: String,
    service_code: String,
    region: String,
    utilization: Option<u8>,
}

impl Quota {
    pub fn new(arn: &str, name: &str, utilization: Option<u8>) -> Result<Self, QuotaError> {
        let (region, account_id, service_code, quota_code) = split_service_quota_arn(arn)?;

        Ok(Self {
            arn: arn.to_string(),
            name: name.to_string(),
            account_id,
            quota_code,
            service_code,
            region,
            utilization,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn arn(&self) -> &str {
        &self.arn
    }

    pub fn account_id(&self) -> &str {
        &self.account_id
    }

    pub fn quota_code(&self) -> &str {
        &self.quota_code
    }

    pub fn service_code(&self) -> &str {
        &self.service_code
    }

    pub fn region(&self) -> &str {
        &self.region
    }

    pub fn utilization(&self) -> Option<u8> {
        self.utilization
    }
}

// split_arn splits a service quota ARN into the fields of interest, returning:
// region, account, service code, quota code
// An Arn is of the form: arn:${Partition}:servicequotas:${Region}:${Account}:${ServiceCode}/${QuotaCode}
fn split_service_quota_arn(arn: &str) -> Result<(String, String, String, String), QuotaError> {
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

    Ok((
        arn_components_vec[3].to_string(), // region
        arn_components_vec[4].to_string(), // account
        arn_components_vec[5].to_string(), // servicecode
        quota_code.to_string(),
    ))
}
