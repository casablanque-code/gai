use gai_core::sim::SourceResolver;
use gai_core::types::{HostsEntry, NssSource, StepResult};
use hickory_resolver::config::{NameServerConfig, Protocol, ResolverConfig, ResolverOpts};
use hickory_resolver::Resolver;
use std::net::{IpAddr, SocketAddr};

/// Answers each NSS source the way the real OS would, for a system that
/// isn't running systemd-resolved (resolv.conf has real nameservers, not
/// the 127.0.0.53 stub). The stub case is handled by `runtime` /
/// `reality`, since it needs a D-Bus round trip rather than a plain query.
pub struct SystemSourceResolver {
    hosts: Vec<HostsEntry>,
    /// Nameservers from resolv.conf as written — meaningless if it's the
    /// systemd-resolved stub (127.0.0.53), in which case queries against
    /// this list would just hit the stub itself.
    nameservers: Vec<IpAddr>,
    /// Real per-link nameservers from resolved's D-Bus Manager.DNS
    /// property. Empty on systems not running systemd-resolved.
    resolved_nameservers: Vec<IpAddr>,
}

impl SystemSourceResolver {
    pub fn new(hosts: Vec<HostsEntry>, nameservers: Vec<IpAddr>) -> Self {
        Self {
            hosts,
            nameservers,
            resolved_nameservers: Vec::new(),
        }
    }

    /// Attaches resolved's real nameserver list, used to answer the
    /// `resolve` NSS source and, when resolv.conf is the stub, to back
    /// plain `dns` queries too — since 127.0.0.53 itself isn't a server
    /// worth asking hickory-resolver to talk to directly.
    pub fn with_resolved_nameservers(mut self, servers: Vec<IpAddr>) -> Self {
        self.resolved_nameservers = servers;
        self
    }

    fn lookup_hosts(&self, name: &str) -> StepResult {
        let addrs: Vec<IpAddr> = self
            .hosts
            .iter()
            .filter(|e| e.names.iter().any(|n| n == name))
            .map(|e| e.ip)
            .collect();
        if addrs.is_empty() {
            StepResult::NotFound
        } else {
            StepResult::Found(addrs)
        }
    }

    fn effective_dns_servers(&self) -> &[IpAddr] {
        if !self.resolved_nameservers.is_empty() {
            &self.resolved_nameservers
        } else {
            &self.nameservers
        }
    }

    fn query(name: &str, servers: &[IpAddr]) -> StepResult {
        if servers.is_empty() {
            return StepResult::Skipped {
                reason: "no nameservers configured".into(),
            };
        }
        let mut cfg = ResolverConfig::new();
        for ns in servers {
            cfg.add_name_server(NameServerConfig::new(
                SocketAddr::new(*ns, 53),
                Protocol::Udp,
            ));
        }
        match Resolver::new(cfg, ResolverOpts::default()) {
            Ok(resolver) => match resolver.lookup_ip(name) {
                Ok(lookup) => {
                    let addrs: Vec<IpAddr> = lookup.iter().collect();
                    if addrs.is_empty() {
                        StepResult::NotFound
                    } else {
                        StepResult::Found(addrs)
                    }
                }
                Err(_) => StepResult::NotFound,
            },
            Err(e) => StepResult::Skipped {
                reason: format!("resolver init failed: {e}"),
            },
        }
    }

    fn lookup_dns(&self, name: &str) -> StepResult {
        Self::query(name, self.effective_dns_servers())
    }

    fn lookup_resolve(&self, name: &str) -> StepResult {
        if self.resolved_nameservers.is_empty() {
            return StepResult::Skipped {
                reason: "systemd-resolved not detected on D-Bus".into(),
            };
        }
        Self::query(name, &self.resolved_nameservers)
    }
}

impl SourceResolver for SystemSourceResolver {
    fn resolve(&mut self, source: &NssSource, name: &str) -> StepResult {
        match source {
            NssSource::Files => self.lookup_hosts(name),
            NssSource::Dns => self.lookup_dns(name),
            // mDNS and myhostname probing land in a follow-up patch, not
            // MVP. Marking as Skipped is honest — it does not pretend the
            // chain continued past a source we can't actually answer for.
            NssSource::Mdns4Minimal
            | NssSource::Mdns6Minimal
            | NssSource::Mdns4
            | NssSource::Mdns6 => StepResult::Skipped {
                reason: "mDNS probing not implemented in MVP".into(),
            },
            NssSource::Myhostname => StepResult::Skipped {
                reason: "myhostname probing not implemented in MVP".into(),
            },
            NssSource::Resolve => self.lookup_resolve(name),
            NssSource::Other(s) => StepResult::Skipped {
                reason: format!("unknown NSS source '{s}'"),
            },
        }
    }
}
