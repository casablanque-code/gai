use hickory_resolver::config::{NameServerConfig, ResolveHosts, ResolverConfig, ResolverOpts};
use hickory_resolver::net::runtime::TokioRuntimeProvider;
use hickory_resolver::Resolver;
use std::net::IpAddr;

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
    let cfg = ResolverConfig::from_parts(
        None,
        vec![],
        servers
            .iter()
            .map(|ip| NameServerConfig::udp(*ip))
            .collect(),
    );
    // Same rationale as SystemSourceResolver::query: hickory-resolver
    // reads /etc/hosts by default regardless of an explicit remote
    // ResolverConfig. The whole point of a reality check is to be an
    // independent signal — with hosts-file lookup left on, a Files-based
    // resolution and this "independent" check would agree by
    // construction, masking the exact discrepancy gai exists to find.
    let opts = ResolverOpts {
        use_hosts_file: ResolveHosts::Never,
        ..ResolverOpts::default()
    };

    // hickory-resolver 0.26+ is async-only (see resolver.rs::query for
    // the full rationale — this crate stays sync-first on purpose, so we
    // bridge with a short-lived current-thread runtime here too).
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    let addresses = rt.block_on(async {
        let resolver = Resolver::builder_with_config(cfg, TokioRuntimeProvider::default())
            .with_options(opts)
            .build()?;
        anyhow::Ok(match resolver.lookup_ip(name).await {
            Ok(lookup) => lookup.iter().collect(),
            Err(_) => Vec::new(),
        })
    })?;

    Ok(RealityCheck {
        queried_servers: servers.to_vec(),
        addresses,
    })
}
