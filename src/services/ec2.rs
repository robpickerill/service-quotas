use crate::util;

pub struct Client {
    client: aws_sdk_ec2::Client,
}

impl Client {
    pub async fn new(region: &str) -> Self {
        let (config, retries) = util::aws_config_with_region(region).await;
        let client_config = aws_sdk_ec2::config::Builder::from(&config)
            .retry_config(retries)
            .build();
        let client = aws_sdk_ec2::Client::from_conf(client_config);

        Self { client }
    }

    pub async fn regions(&self) -> Vec<String> {
        let result = self.client.describe_regions().send().await.unwrap();

        result
            .regions()
            .unwrap()
            .iter()
            .map(|r| r.region_name().unwrap().to_string())
            .collect()
    }
}
