use hickory_resolver::config::{NameServerConfig, Protocol, ResolverConfig, ResolverOpts};
use hickory_resolver::Resolver;
use std::net::{IpAddr, SocketAddr};

/// Result of an independent, out-of-band DNS query used purely as a
/// sanity check against the simulated nsswitch outcome.
///
/// Deliberately queries the *system's actual configured nameservers*
/// (or systemd-resolved's per-link servers, once that lookup lands),
/// not a hardcoded 8.8.8.8 — a hardcoded public resolver would produce
/// false "reality gaps" on any split-horizon/VPN/corporate DNS setup.
pub struct RealityCheck {
    pub queried_servers: Vec<IpAddr>,
    pub addresses: Vec<IpAddr>,
}

pub fn check(name: &str, servers: &[IpAddr]) -> anyhow::Result<RealityCheck> {
    let mut cfg = ResolverConfig::new();
    for ns in servers {
        cfg.add_name_server(NameServerConfig::new(
            SocketAddr::new(*ns, 53),
            Protocol::Udp,
        ));
    }
    let resolver = Resolver::new(cfg, ResolverOpts::default())?;
    let addresses = match resolver.lookup_ip(name) {
        Ok(lookup) => lookup.iter().collect(),
        Err(_) => Vec::new(),
    };

    Ok(RealityCheck {
        queried_servers: servers.to_vec(),
        addresses,
    })
}
