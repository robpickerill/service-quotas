mod cli;
mod quota;
mod services;
mod util;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// the threshold by which to alert on for utilization of a service quota
    #[arg(short, long, default_value_t = 75)]
    threshold: u8,

    #[arg(short, long, num_args(0..))]
    regions: Vec<String>,
}
#[tokio::main]
async fn main() {
    let args = Args::parse();
    cli::run(args).await;
}
