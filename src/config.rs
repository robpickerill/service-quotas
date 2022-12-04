use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct Config {
    threshold: u8,
    regions: Vec<String>,
    ignored_quotas: HashSet<String>,
}

impl Config {
    pub fn new(args: &clap::ArgMatches) -> Self {
        let regions = regions(args);
        let threshold = threshold(args);
        let ignored_quotas = ignored_quotas(args);

        Self {
            threshold,
            regions,
            ignored_quotas,
        }
    }

    pub fn threshold(&self) -> u8 {
        self.threshold
    }

    pub fn regions(&self) -> &Vec<String> {
        &self.regions
    }

    pub fn ignored_quotas(&self) -> &HashSet<String> {
        &self.ignored_quotas
    }
}

fn regions(args: &clap::ArgMatches) -> Vec<String> {
    let regions = args
        .try_get_many::<String>("regions")
        .ok()
        .flatten()
        .unwrap_or_default()
        .cloned()
        .collect::<Vec<_>>();

    if regions.is_empty() {
        vec!["us-east-1".to_string()]
    } else {
        regions
    }
}

fn threshold(args: &clap::ArgMatches) -> u8 {
    args.try_get_one::<u8>("threshold")
        .ok()
        .flatten()
        .unwrap_or(&75)
        .to_owned()
}

fn ignored_quotas(args: &clap::ArgMatches) -> HashSet<String> {
    args.try_get_many::<String>("ignore")
        .ok()
        .flatten()
        .unwrap_or_default()
        .cloned()
        .collect::<HashSet<_>>()
}
