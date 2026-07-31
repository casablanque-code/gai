use gai_core::config::{parse_hosts, parse_nsswitch, parse_resolv_conf};
use gai_core::platform::paths;
use gai_core::sim::simulate;
use gai_probe::resolved::effective_addresses;
use gai_probe::{query_nameservers, reality, SystemSourceResolver};
use std::net::IpAddr;
use std::path::Path;

/// Two address lists represent the same answer regardless of order — the
/// order in which a resolver happens to return a set of A/AAAA records
/// carries no meaning, so comparing them as ordered Vecs produces false
/// "disagreement" diagnoses (e.g. localhost's `/etc/hosts` entry vs a DNS
/// reality check both returning {127.0.0.1, ::1} in different orders).
fn same_address_set(a: &[IpAddr], b: &[IpAddr]) -> bool {
    let mut a = a.to_vec();
    let mut b = b.to_vec();
    a.sort();
    b.sort();
    a == b
}

pub fn run(name: &str) -> anyhow::Result<()> {
    let nss = parse_nsswitch(Path::new(paths::NSSWITCH_CONF))?;
    let resolv = parse_resolv_conf(Path::new(paths::RESOLV_CONF))?;
    let hosts = parse_hosts(Path::new(paths::HOSTS))?;
    let resolved_servers = effective_addresses(&query_nameservers().unwrap_or_default());

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
    if !outcome.resolved() {
        if halted_early {
            match &reality_result {
                Some(reality) if !reality.addresses.is_empty() => {
                    println!(
                        "  The simulated OS chain never reached DNS — it halted earlier in \
                         nsswitch.conf. A direct DNS query against the same nameservers \
                         succeeded: {:?}",
                        reality.addresses
                    );
                    println!("  FIX: review the [NOTFOUND=return] rule that stopped the chain.");
                }
                Some(_) => {
                    println!(
                        "  The simulated OS chain halted early in nsswitch.conf, but a \
                         direct DNS query also found nothing. No discrepancy — this name \
                         likely doesn't exist anywhere."
                    );
                }
                None => {
                    println!(
                        "  The simulated OS chain halted early in nsswitch.conf. No reality \
                         check was possible (no nameservers configured or the query failed), \
                         so this could not be cross-checked against DNS."
                    );
                }
            }
        } else {
            match &reality_result {
                None => println!(
                    "  Resolution failed and no nameservers were configured to cross-check."
                ),
                Some(reality) if reality.addresses.is_empty() => println!(
                    "  Resolution failed through the full chain, and a direct DNS query \
                     agrees — this name doesn't resolve."
                ),
                Some(reality) => println!(
                    "  Resolution failed through the simulated chain, but a direct DNS \
                     query found {:?}. Something in the chain (not covered by an \
                     early-halt rule) is suppressing a real answer.",
                    reality.addresses
                ),
            }
        }
    } else {
        match &reality_result {
            None => println!(
                "  Resolved to {:?} via the simulated chain, but no reality check was \
                 possible (no nameservers configured or the query failed) — take this \
                 result on trust rather than as cross-checked.",
                outcome.final_addresses
            ),
            Some(reality) if same_address_set(&reality.addresses, &outcome.final_addresses) => {
                println!("  Resolution succeeded and matches direct DNS. No discrepancy found.")
            }
            Some(reality)
                if outcome
                    .steps
                    .last()
                    .is_some_and(|s| s.source == gai_core::types::NssSource::Dns) =>
            {
                println!(
                    "  The OS chain and a direct DNS query disagree: {:?} vs {:?}. Both \
                     answers came from DNS itself, so this isn't an earlier source (files/mdns) \
                     intercepting the name — it's more likely two separate queries landing on \
                     different anycast/GeoDNS edges, or a change between queries. If this \
                     persists for a name that should be static, that's worth digging into \
                     further.",
                    outcome.final_addresses, reality.addresses
                )
            }
            Some(reality) => println!(
                "  The OS chain and a direct DNS query disagree: {:?} vs {:?}. \
                 Something earlier in the chain (files/mdns) is answering instead of DNS.",
                outcome.final_addresses, reality.addresses
            ),
        }
    }

    Ok(())
}
