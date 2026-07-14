use gai_core::config::{parse_hosts, parse_nsswitch, parse_resolv_conf};
use gai_core::platform::paths;
use gai_core::sim::simulate;
use gai_probe::resolved::flat_addresses;
use gai_probe::{query_nameservers, reality, SystemSourceResolver};
use std::path::Path;

pub fn run(name: &str) -> anyhow::Result<()> {
    let nss = parse_nsswitch(Path::new(paths::NSSWITCH_CONF))?;
    let resolv = parse_resolv_conf(Path::new(paths::RESOLV_CONF))?;
    let hosts = parse_hosts(Path::new(paths::HOSTS))?;
    let resolved_servers = flat_addresses(&query_nameservers().unwrap_or_default());

    println!("[gai] Simulating name resolution for \"{name}\"...\n");

    let mut resolver = SystemSourceResolver::new(hosts.clone(), resolv.nameservers.clone())
        .with_resolved_nameservers(resolved_servers.clone());
    let outcome = simulate(&nss, name, &mut resolver);

    // Reality check: query the servers that actually answer for this
    // system — resolved's real per-link servers if the stub is in play,
    // otherwise whatever resolv.conf lists directly. Bypasses NSS
    // entirely, so it's an independent comparison, not a repeat of the
    // simulated path.
    let effective_servers = if resolv.is_systemd_stub && !resolved_servers.is_empty() {
        &resolved_servers
    } else {
        &resolv.nameservers
    };
    let reality_result = if effective_servers.is_empty() {
        None
    } else {
        reality::check(name, effective_servers).ok()
    };

    println!(
        "  (reality check via {:?}, systemd-resolved stub: {})\n",
        effective_servers, resolv.is_systemd_stub
    );

    println!("RESOLUTION PATH (simulated):");
    for (i, step) in outcome.steps.iter().enumerate() {
        println!("  {}. [{:?}] {:?}", i + 1, step.source, step.result);
    }

    let halted_early = outcome
        .steps
        .last()
        .map(|s| s.halted_chain.is_some())
        .unwrap_or(false)
        && !outcome.resolved();

    println!("\nDIAGNOSIS:");
    match (&reality_result, outcome.resolved(), halted_early) {
        (Some(reality), false, true) if !reality.addresses.is_empty() => {
            println!(
                "  The simulated OS chain never reached DNS — it halted earlier in \
                 nsswitch.conf. A direct DNS query against the same nameservers \
                 succeeded: {:?}",
                reality.addresses
            );
            println!("  FIX: review the [NOTFOUND=return] rule that stopped the chain.");
        }
        (Some(reality), true, _) if reality.addresses != outcome.final_addresses => {
            println!(
                "  The OS chain and a direct DNS query disagree: {:?} vs {:?}. \
                 Something earlier in the chain (files/mdns) is answering \
                 instead of DNS.",
                outcome.final_addresses, reality.addresses
            );
        }
        (_, true, _) => {
            println!("  Resolution succeeded and matches direct DNS. No discrepancy found.");
        }
        (None, false, _) => {
            println!("  Resolution failed and no nameservers were configured to cross-check.");
        }
        _ => {
            println!("  No discrepancy detected between the simulated path and reality.");
        }
    }

    Ok(())
}
