use env_logger::Env;
use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};

#[macro_use]
extern crate log;

#[derive(Debug)]
enum CliError {
    Runtime(String),
    UnknownSubcommand,
}

impl Error for CliError {}
impl Display for CliError {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            Self::Runtime(s) => write!(f, "RuntimeError: {}", s),
            Self::UnknownSubcommand => write!(f, "UnknownSubcommand"),
        }
    }
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(
        Env::default().default_filter_or("info,aws_config=error,aws_http=error"),
    )
    .init();

    let args = service_quotas::cli::new().get_matches();

    let result = match args.subcommand() {
        None => Err(CliError::UnknownSubcommand),
        Some(("utilization", args)) => service_quotas::utilization()
            .await
            .map_err(|e| CliError::Runtime(e.to_string())),
        Some(("list-quotas", args)) => service_quotas::list_quotas()
            .await
            .map_err(|e| CliError::Runtime(e.to_string())),
        _ => Err(CliError::UnknownSubcommand),
    };

    if let Err(err) = result {
        error!("Error: {}", err);
        std::process::exit(1);
    }
}
