#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Quota {
    name: String,
    quota_code: String,
    service_code: String,
    region: String,
    utilization: Option<u8>,
}

impl Quota {
    pub fn new(
        name: &str,
        service_code: &str,
        quota_code: &str,
        region: &str,
        utilization: Option<u8>,
    ) -> Self {
        Self {
            name: name.to_string(),
            quota_code: quota_code.to_string(),
            service_code: service_code.to_string(),
            region: region.to_string(),
            utilization,
        }
    }
}
