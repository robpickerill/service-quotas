mod cloudwatch;
mod service;
mod util;

#[tokio::main]
async fn main() {
    let ec2 = service::Client::new().await;
    let result = ec2.quotas("ec2").await;
    println!("{:?}", result)
}
