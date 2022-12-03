mod cli;
mod notifiers;
mod quota;
mod services;
mod util;

use clap::Parser;
use env_logger::Env;

#[macro_use]
extern crate log;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(
        short,
        long,
        default_value_t = 75,
        help = "the threshold to alert at for utlization of a service quota"
    )]
    threshold: u8,

    #[arg(short, long, num_args(0..), help="the AWS region(s) to check quotas for, defaults to all AWS regions")]
    regions: Vec<String>,
}
#[tokio::main]
async fn main() {
    let args = Args::parse();

    env_logger::Builder::from_env(Env::default().default_filter_or("info,aws_config=error")).init();
    info!("threshold: {}", args.threshold);
    info!("regions: {}", args.regions.join(", "));

    cli::run(args).await.unwrap();
}
