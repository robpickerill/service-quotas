#[derive(Debug, Clone)]
pub struct ServiceQuota {
    name: String,
    service_code: String,
    utilization: Option<u8>,
}

impl ServiceQuota {
    pub fn new(name: &str, service_code: &str, utilization: Option<u8>) -> Self {
        Self {
            name: name.to_string(),
            service_code: service_code.to_string(),
            utilization: utilization,
        }
    }

    pub fn utilization(self) -> Option<u8> {
        self.utilization
    }
}
