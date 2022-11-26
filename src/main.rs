mod cli;
mod quota;
mod services;
mod util;

use clap::Parser;

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
    cli::run(args).await;
}
