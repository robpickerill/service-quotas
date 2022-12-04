use clap::{Arg, Command};

pub fn new() -> Command {
    Command::new(env!("CARGO_PKG_NAME"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .version(env!("CARGO_PKG_VERSION"))
        .propagate_version(true)
        .subcommands([utilization(), list_quotas()])
}

fn common_args() -> Vec<Arg> {
    vec![Arg::new("regions")
        .short('r')
        .long("regions")
        .num_args(1..)
        .help("The AWS region(s) to check quotas for, defaults to us-east-1")]
}

fn list_quotas() -> Command {
    Command::new("list-quotas")
        .about("List all quotas")
        .args(&common_args())
}

fn utilization() -> Command {
    Command::new("utilization")
        .about("Check utilization of quotas")
        .args(&common_args())
        .args(vec![
            Arg::new("threshold")
                .short('t')
                .long("threshold")
                .help("The threshold to alert at for utlization of a service quota"),
            Arg::new("ignore")
                .short('i')
                .long("ignore")
                .num_args(1..)
                .help("The quota codes to ignore, defaults to none"),
        ])
}
