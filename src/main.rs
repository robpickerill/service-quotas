use env_logger::Env;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info,aws_config=error")).init();

    let args = service_quotas::cli::new().get_matches();

    match args.subcommand() {
        None => {
            println!("No subcommand was used");
            std::process::exit(1);
        }
        Some(("utilization", args)) => service_quotas::utilization(args).await.unwrap(),
        Some(("list-quotas", args)) => service_quotas::list_quotas(args).await.unwrap(),
        _ => unimplemented!("subcommands not yet implemented"),
    };
}
