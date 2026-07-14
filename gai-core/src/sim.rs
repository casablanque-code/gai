use crate::types::{
    NssAction, NssCriterion, NssSource, NssStatus, NsswitchConfig, ResolutionStep, StepResult,
};
use std::net::IpAddr;

/// Supplies the actual lookup result for one NSS source. gai-core doesn't
/// do I/O itself — gai-probe implements this trait with real DNS sockets,
/// /etc/hosts lookups, mDNS queries, etc. This keeps the decision logic
/// testable against fixtures without a network in sight.
pub trait SourceResolver {
    fn resolve(&mut self, source: &NssSource, name: &str) -> StepResult;
}

pub struct SimulationOutcome {
    pub steps: Vec<ResolutionStep>,
    pub final_addresses: Vec<IpAddr>,
}

impl SimulationOutcome {
    pub fn resolved(&self) -> bool {
        !self.final_addresses.is_empty()
    }
}

/// Walks the `hosts:` chain from nsswitch.conf in order, applying
/// [STATUS=action] criteria exactly as glibc's NSS dispatcher does:
/// each source is tried, its result is classified into a status, and if
/// that status has a matching criterion the configured action either
/// halts the chain (`return`) or falls through to the next source
/// (`continue`, the default when no criterion matches at all).
pub fn simulate(
    config: &NsswitchConfig,
    name: &str,
    resolver: &mut impl SourceResolver,
) -> SimulationOutcome {
    let mut steps = Vec::new();
    let mut final_addresses = Vec::new();

    for entry in &config.hosts {
        let result = resolver.resolve(&entry.source, name);
        let status = classify(&result);
        let matched_criterion = entry
            .criteria
            .iter()
            .find(|c| c.status == status)
            .cloned();

        let halts = match &matched_criterion {
            Some(NssCriterion {
                action: NssAction::Return,
                ..
            }) => true,
            // Default glibc behavior without an explicit criterion:
            // SUCCESS always halts the chain.
            None => status == NssStatus::Success,
            _ => false,
        };

        if let StepResult::Found(ref addrs) = result {
            final_addresses = addrs.clone();
        }

        steps.push(ResolutionStep {
            source: entry.source.clone(),
            result,
            halted_chain: if halts { matched_criterion } else { None },
        });

        if halts {
            break;
        }
    }

    SimulationOutcome {
        steps,
        final_addresses,
    }
}

fn classify(result: &StepResult) -> NssStatus {
    match result {
        StepResult::Found(_) => NssStatus::Success,
        StepResult::NotFound => NssStatus::NotFound,
        StepResult::Skipped { .. } => NssStatus::Unavail,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{NssAction, NssCriterion, NssEntry, NssStatus};

    /// A resolver double for tests: scripted answers per source.
    struct ScriptedResolver {
        answers: Vec<(NssSource, StepResult)>,
    }

    impl SourceResolver for ScriptedResolver {
        fn resolve(&mut self, source: &NssSource, _name: &str) -> StepResult {
            self.answers
                .iter()
                .find(|(s, _)| s == source)
                .map(|(_, r)| r.clone())
                .unwrap_or(StepResult::NotFound)
        }
    }

    #[test]
    fn notfound_return_halts_before_dns_is_ever_reached() {
        let config = NsswitchConfig {
            hosts: vec![
                NssEntry {
                    source: NssSource::Files,
                    criteria: vec![],
                },
                NssEntry {
                    source: NssSource::Mdns4Minimal,
                    criteria: vec![NssCriterion {
                        status: NssStatus::NotFound,
                        action: NssAction::Return,
                    }],
                },
                NssEntry {
                    source: NssSource::Dns,
                    criteria: vec![],
                },
            ],
        };
        let mut resolver = ScriptedResolver {
            answers: vec![
                (NssSource::Files, StepResult::NotFound),
                (NssSource::Mdns4Minimal, StepResult::NotFound),
                (
                    NssSource::Dns,
                    StepResult::Found(vec!["192.168.1.50".parse().unwrap()]),
                ),
            ],
        };

        let outcome = simulate(&config, "api.local", &mut resolver);

        assert_eq!(outcome.steps.len(), 2, "dns step must never be reached");
        assert!(!outcome.resolved());
        assert!(outcome.steps[1].halted_chain.is_some());
    }

    #[test]
    fn success_without_criterion_still_halts_by_default() {
        let config = NsswitchConfig {
            hosts: vec![
                NssEntry {
                    source: NssSource::Files,
                    criteria: vec![],
                },
                NssEntry {
                    source: NssSource::Dns,
                    criteria: vec![],
                },
            ],
        };
        let mut resolver = ScriptedResolver {
            answers: vec![(
                NssSource::Files,
                StepResult::Found(vec!["127.0.0.1".parse().unwrap()]),
            )],
        };

        let outcome = simulate(&config, "localhost", &mut resolver);

        assert_eq!(outcome.steps.len(), 1);
        assert!(outcome.resolved());
    }
}
