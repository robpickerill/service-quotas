mod cli;

use env_logger::Env;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info,aws_config=error")).init();

    let args = cli::new().get_matches();

    match args.subcommand() {
        None => service_quotas::run(&args).await.unwrap(),
        _ => unimplemented!("subcommands not yet implemented"),
    };
}
