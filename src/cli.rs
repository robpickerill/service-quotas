// use clap::Parser;

// #[derive(Parser, Debug)]
// #[command(author, version, about, long_about = None)]
// pub struct Args {
//     #[arg(
//         short,
//         long,
//         default_value_t = 75,
//         help = "the threshold to alert at for utlization of a service quota"
//     )]
//     pub threshold: u8,

//     #[arg(short, long, num_args(0..), help="the AWS region(s) to check quotas for, defaults to all AWS regions")]
//     pub regions: Vec<String>,
// }

use clap::{Arg, Command};

pub fn new() -> Command {
    Command::new(env!("CARGO_PKG_NAME"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .version(env!("CARGO_PKG_VERSION"))
        .propagate_version(true)
        .args(common_args())
}

fn common_args() -> Vec<Arg> {
    vec![
        Arg::new("threshold")
            .short('t')
            .long("threshold")
            .help("the threshold to alert at for utlization of a service quota"),
        Arg::new("regions")
            .short('r')
            .long("regions")
            .num_args(1..)
            .help("the AWS region(s) to check quotas for, defaults to all AWS regions"),
    ]
}
