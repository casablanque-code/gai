use gai_core::config::parse_nsswitch;
use gai_core::platform::paths;
use gai_core::sim::simulate;
use gai_core::types::StepResult;
use gai_probe::resolved::effective_addresses;
use gai_probe::{query_nameservers, SystemSourceResolver};
use std::path::Path;

use crate::style::{format_addrs, panel, Style, GREEN, YELLOW};

pub fn run(name: &str) -> anyhow::Result<()> {
    let style = Style::detect();
    let nss = parse_nsswitch(Path::new(paths::NSSWITCH_CONF))?;
    let resolv = gai_core::config::parse_resolv_conf(Path::new(paths::RESOLV_CONF))?;
    let hosts = gai_core::config::parse_hosts(Path::new(paths::HOSTS))?;

    let resolved_servers = effective_addresses(&query_nameservers().unwrap_or_default());

    if resolv.is_systemd_stub {
        if resolved_servers.is_empty() {
            println!("note: resolv.conf points at the systemd-resolved stub (127.0.0.53),");
            println!("      but resolved's D-Bus API didn't return any nameservers.\n");
        } else {
            println!("note: resolv.conf points at the systemd-resolved stub (127.0.0.53);");
            println!(
                "      real nameservers via D-Bus: {}\n",
                format_addrs(&resolved_servers)
            );
        }
    }

    let mut resolver = SystemSourceResolver::new(hosts, resolv.nameservers)
        .with_resolved_nameservers(resolved_servers);
    let outcome = simulate(&nss, name, &mut resolver);

    println!("Resolution path for \"{name}\":\n");
    for (i, step) in outcome.steps.iter().enumerate() {
        let label = format!("{:?}", step.source);
        let (tag, detail) = match &step.result {
            StepResult::Found(addrs) => (style.green("FOUND"), format_addrs(addrs)),
            StepResult::NotFound => (style.dim("NOT FOUND"), String::new()),
            StepResult::Skipped { reason } => (style.red("SKIPPED"), reason.clone()),
        };
        print!("  {}. {:<14} {}", i + 1, label, tag);
        if !detail.is_empty() {
            print!("  {detail}");
        }
        println!();
        if let Some(criterion) = &step.halted_chain {
            println!(
                "     {} [{:?}={:?}]",
                style.dim("chain halted:"),
                criterion.status,
                criterion.action
            );
        }
    }
    println!();

    if outcome.resolved() {
        panel(
            &style,
            "Result:",
            &[format_addrs(&outcome.final_addresses)],
            GREEN,
        );
    } else {
        panel(&style, "Result:", &["not resolved".to_string()], YELLOW);
    }

    Ok(())
}
