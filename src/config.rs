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
    if let Ok(Some(regions)) = args.clone().try_remove_many("regions") {
        regions.collect::<Vec<String>>()
    } else {
        vec!["us-east-1".to_string()]
    }
}

fn threshold(args: &clap::ArgMatches) -> u8 {
    if let Ok(Some(threshold)) = args.clone().try_remove_one::<u8>("threshold") {
        threshold
    } else {
        75
    }
}

fn ignored_quotas(args: &clap::ArgMatches) -> HashSet<String> {
    if let Ok(Some(ignored_quotas)) = args.clone().try_remove_many("ignore") {
        ignored_quotas.collect::<HashSet<String>>()
    } else {
        HashSet::new()
    }
}

#[cfg(test)]
mod test {
    use crate::cli;

    use super::*;

    #[test]
    fn test_region_default() {
        let args = clap::ArgMatches::default();
        let regions = regions(&args);
        assert_eq!(regions, vec!["us-east-1".to_string()]);
    }

    #[test]
    #[ignore = "TODO: fix this test"]
    fn test_region_override() {
        let args = cli::new().get_matches_from(vec![
            "service-quotas",
            "utilization",
            "-r",
            "us-east-1",
            "us-east-2",
            "us-west-1",
            "us-west-2",
        ]);
        let regions = regions(&args);
        assert_eq!(
            regions,
            vec![
                "us-east-1".to_string(),
                "us-east-2".to_string(),
                "us-west-1".to_string(),
                "us-west-2".to_string()
            ]
        );
    }

    #[test]
    fn test_threshold_default() {
        let args = clap::ArgMatches::default();
        let threshold = threshold(&args);
        assert_eq!(threshold, 75);
    }

    #[test]
    #[ignore = "TODO: fix this test"]
    fn test_threshold_override() {
        let args = cli::new().get_matches_from(vec!["service-quotas", "utilization", "-t", "50"]);
        let threshold = threshold(&args);
        assert_eq!(threshold, 50);
    }

    #[test]
    fn test_ignored_quotas_default() {
        let args = clap::ArgMatches::default();
        let ignored_quotas = ignored_quotas(&args);
        assert_eq!(ignored_quotas, HashSet::new());
    }

    #[test]
    #[ignore = "TODO: fix this test"]
    fn test_ignored_quotas_override() {
        let args =
            cli::new().get_matches_from(vec!["service-quotas", "utilization", "-i", "test1"]);
        let ignored_quotas = ignored_quotas(&args);
        assert_eq!(
            ignored_quotas,
            vec!["test1".to_string()].into_iter().collect()
        );
    }
}
