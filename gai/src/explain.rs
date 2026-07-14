use gai_core::config::parse_nsswitch;
use gai_core::platform::paths;
use gai_core::sim::simulate;
use gai_core::types::StepResult;
use gai_probe::resolved::flat_addresses;
use gai_probe::{query_nameservers, SystemSourceResolver};
use std::path::Path;

pub fn run(name: &str) -> anyhow::Result<()> {
    let nss = parse_nsswitch(Path::new(paths::NSSWITCH_CONF))?;
    let resolv = gai_core::config::parse_resolv_conf(Path::new(paths::RESOLV_CONF))?;
    let hosts = gai_core::config::parse_hosts(Path::new(paths::HOSTS))?;

    let resolved_servers = flat_addresses(&query_nameservers().unwrap_or_default());

    if resolv.is_systemd_stub {
        if resolved_servers.is_empty() {
            println!("note: resolv.conf points at the systemd-resolved stub (127.0.0.53),");
            println!("      but resolved's D-Bus API didn't return any nameservers.\n");
        } else {
            println!("note: resolv.conf points at the systemd-resolved stub (127.0.0.53);");
            println!("      real nameservers via D-Bus: {resolved_servers:?}\n");
        }
    }

    let mut resolver = SystemSourceResolver::new(hosts, resolv.nameservers)
        .with_resolved_nameservers(resolved_servers);
    let outcome = simulate(&nss, name, &mut resolver);

    println!("Resolution path for \"{name}\":\n");
    for (i, step) in outcome.steps.iter().enumerate() {
        let status = match &step.result {
            StepResult::Found(addrs) => format!("FOUND {addrs:?}"),
            StepResult::NotFound => "NOT FOUND".to_string(),
            StepResult::Skipped { reason } => format!("SKIPPED ({reason})"),
        };
        println!("  {}. [{:?}] {status}", i + 1, step.source);
        if let Some(criterion) = &step.halted_chain {
            println!(
                "     -> chain halted: [{:?}={:?}]",
                criterion.status, criterion.action
            );
        }
    }

    if outcome.resolved() {
        println!("\nResult: {:?}", outcome.final_addresses);
    } else {
        println!("\nResult: not resolved");
    }

    Ok(())
}
