mod servicequotas;
use clap::Parser;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)] // Read from `Cargo.toml`
struct Cli {}

#[tokio::main]
async fn main() {
    let _ = Cli::parse();

    let sq = servicequotas::Client::new().await;
    let results = sq.get_quotas().await;
    results.map_or_else(
        |e| println!("{:?}", e),
        |r| {
            r.into_iter().for_each(|s| {
                if s.usage_metric().is_some() {
                    println!("{:?}: {:?}", s.service_name(), s.usage_metric());
                }
            })
        },
    );
}
