//! Integration-level scenarios: real config text in, real simulate() out.
//! These are the "does this tool actually do the thing" tests — no live
//! network, no live filesystem beyond a temp fixture, fully deterministic,
//! safe to run in CI on every push.

use gai_core::config::parse_nsswitch;
use gai_core::sim::{simulate, SourceResolver};
use gai_core::types::{NssSource, StepResult};
use std::io::Write;
use std::net::IpAddr;

/// Scripted answers, same pattern as gai-core's unit tests but exercised
/// here against a parsed-from-text nsswitch.conf instead of a
/// hand-built NsswitchConfig struct.
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

fn write_nsswitch(contents: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(contents.as_bytes()).unwrap();
    f
}

/// The scenario from the original project spec: `curl api.local` fails
/// not because the domain doesn't exist, but because mdns4_minimal's
/// [NOTFOUND=return] rule stops the chain before DNS is ever asked, even
/// though a direct DNS query would have succeeded.
#[test]
fn scenario_mdns_notfound_return_hides_a_real_dns_answer() {
    let fixture = write_nsswitch("hosts: files mdns4_minimal [NOTFOUND=return] dns\n");
    let config = parse_nsswitch(fixture.path()).expect("valid nsswitch.conf");

    let mut resolver = ScriptedResolver {
        answers: vec![
            (NssSource::Files, StepResult::NotFound),
            (NssSource::Mdns4Minimal, StepResult::NotFound),
            // dns would have answered — but per the spec, it must never
            // be asked.
            (
                NssSource::Dns,
                StepResult::Found(vec!["192.168.1.50".parse().unwrap()]),
            ),
        ],
    };

    let outcome = simulate(&config, "api.local", &mut resolver);

    assert_eq!(
        outcome.steps.len(),
        2,
        "dns must never be reached — this is the whole point of the tool"
    );
    assert!(!outcome.resolved(), "getaddrinfo() reports failure here");
    assert!(outcome.steps[1].halted_chain.is_some());

    // What a naive reading of the config would miss: DNS was never even
    // tried, despite `dns` being listed right there in nsswitch.conf.
    let reached_dns = outcome.steps.iter().any(|s| s.source == NssSource::Dns);
    assert!(!reached_dns);
}

/// The boring, correct case: minimal nsswitch.conf (`files dns`, no
/// traps), name resolves via the second source since files doesn't
/// have it. This is deliberately the "nothing interesting happened"
/// baseline — it should stay boring.
#[test]
fn scenario_plain_files_then_dns_no_traps() {
    let fixture = write_nsswitch("hosts: files dns\n");
    let config = parse_nsswitch(fixture.path()).expect("valid nsswitch.conf");

    let mut resolver = ScriptedResolver {
        answers: vec![
            (NssSource::Files, StepResult::NotFound),
            (
                NssSource::Dns,
                StepResult::Found(vec!["93.184.216.34".parse().unwrap()]),
            ),
        ],
    };

    let outcome = simulate(&config, "example.com", &mut resolver);

    assert_eq!(outcome.steps.len(), 2);
    assert!(outcome.resolved());
    assert_eq!(
        outcome.final_addresses,
        vec!["93.184.216.34".parse::<IpAddr>().unwrap()]
    );
}

/// A comment-heavy, multi-database nsswitch.conf, like a real machine's,
/// to make sure the parser only reacts to the `hosts:` line and ignores
/// everything else — the exact file shape that broke naive parsers in
/// early testing.
#[test]
fn scenario_realistic_multiline_nsswitch_conf() {
    let fixture = write_nsswitch(
        "# /etc/nsswitch.conf\n\
         #\n\
         passwd:         files systemd\n\
         group:          files systemd\n\
         shadow:         files\n\
         gshadow:        files\n\
         \n\
         hosts:          files mdns4_minimal [NOTFOUND=return] dns myhostname # trailing note\n\
         networks:       files\n\
         \n\
         protocols:      db files\n\
         services:       db files\n\
         ethers:         db files\n\
         rpc:            db files\n",
    );
    let config = parse_nsswitch(fixture.path()).expect("valid nsswitch.conf");

    assert_eq!(config.hosts.len(), 4, "files, mdns4_minimal, dns, myhostname");
    assert_eq!(config.hosts[0].source, NssSource::Files);
    assert_eq!(config.hosts[1].source, NssSource::Mdns4Minimal);
    assert_eq!(config.hosts[2].source, NssSource::Dns);
    assert_eq!(config.hosts[3].source, NssSource::Myhostname);
}
