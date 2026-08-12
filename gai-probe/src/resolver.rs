use gai_core::sim::SourceResolver;
use gai_core::types::{HostsEntry, NssSource, StepResult};
use hickory_resolver::config::{NameServerConfig, ResolveHosts, ResolverConfig, ResolverOpts};
use hickory_resolver::net::runtime::TokioRuntimeProvider;
use hickory_resolver::Resolver;
use std::net::IpAddr;

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
        let cfg = ResolverConfig::from_parts(
            None,
            vec![],
            servers
                .iter()
                .map(|ip| NameServerConfig::udp(*ip))
                .collect(),
        );
        // Critical: hickory-resolver consults /etc/hosts by default even
        // with an explicit remote-only ResolverConfig. Left enabled, this
        // "dns" source would silently double as a Files lookup, which is
        // exactly the layering confusion gai exists to expose. Disabled
        // so this function is a pure, wire-level DNS query.
        let mut opts = ResolverOpts::default();
        opts.use_hosts_file = ResolveHosts::Never;

        // hickory-resolver 0.26 dropped the blocking `Resolver` — it's
        // tokio-async only now (this is also the version bump that fixes
        // RUSTSEC-2026-0119's O(n^2) message-encoding DoS in
        // hickory-proto 0.24.x). SourceResolver::resolve is a sync trait
        // method by design (it's called from gai-core's plain simulation
        // loop), so we bridge with a short-lived current-thread runtime
        // rather than making the whole crate async for what's a couple
        // of one-shot queries per CLI invocation.
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                return StepResult::Skipped {
                    reason: format!("failed to start async runtime: {e}"),
                }
            }
        };

        rt.block_on(async {
            let resolver = match Resolver::builder_with_config(cfg, TokioRuntimeProvider::default())
                .with_options(opts)
                .build()
            {
                Ok(r) => r,
                Err(e) => {
                    return StepResult::Skipped {
                        reason: format!("resolver init failed: {e}"),
                    }
                }
            };
            match resolver.lookup_ip(name).await {
                Ok(lookup) => {
                    let addrs: Vec<IpAddr> = lookup.iter().collect();
                    if addrs.is_empty() {
                        StepResult::NotFound
                    } else {
                        StepResult::Found(addrs)
                    }
                }
                // A query error (timeout, refused, network unreachable,
                // ...) is not the same fact as an authoritative NXDOMAIN,
                // and diagnosis logic downstream treats them very
                // differently: `NotFound` is read as "DNS was asked and
                // has no answer", while an error means DNS was never
                // successfully asked at all. Collapsing the two here
                // previously caused doctor to falsely diagnose "the OS
                // chain disagrees with DNS reality" (or, symmetrically,
                // "no discrepancy") on a plain timeout/flaky query rather
                // than surfacing that the check itself didn't run. Mirrors
                // how lookup_mdns/lookup_mdns6 already handle their own
                // query errors below.
                // A query error (timeout, refused, network unreachable,
                // ...) is not the same fact as an authoritative NXDOMAIN,
                // and diagnosis logic downstream treats them very
                // differently: `NotFound` is read as "DNS was asked and
                // has no answer", while an error means DNS was never
                // successfully asked at all. `NetError::is_no_records_found`
                // is hickory's own way of distinguishing an authoritative
                // negative answer (NXDOMAIN/NODATA) from every other kind
                // of failure, so a legitimate "doesn't exist" still maps
                // to NotFound while everything else surfaces as Skipped.
                // Mirrors how lookup_mdns/lookup_mdns6 already handle
                // their own query errors below.
                Err(e) if e.is_no_records_found() => StepResult::NotFound,
                Err(e) => StepResult::Skipped {
                    reason: format!("DNS query failed: {e}"),
                },
            }
        })
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

    /// One-shot mDNS A-record query on 224.0.0.251:5353, ~500ms deadline.
    /// This is what mdns4_minimal actually does under the hood — a single
    /// best-effort broadcast, not a persistent responder.
    fn lookup_mdns(name: &str) -> StepResult {
        if let Some(reason) = Self::skip_non_local(name) {
            return StepResult::Skipped { reason };
        }
        match crate::mdns::query_a_record(name) {
            Ok(addrs) if !addrs.is_empty() => StepResult::Found(addrs),
            Ok(_) => StepResult::NotFound,
            Err(e) => StepResult::Skipped {
                reason: format!("mDNS query failed: {e}"),
            },
        }
    }

    /// Same as `lookup_mdns` but for AAAA (IPv6) records, backing
    /// mdns6/mdns6_minimal.
    fn lookup_mdns6(name: &str) -> StepResult {
        if let Some(reason) = Self::skip_non_local(name) {
            return StepResult::Skipped { reason };
        }
        match crate::mdns::query_aaaa_record(name) {
            Ok(addrs) if !addrs.is_empty() => StepResult::Found(addrs),
            Ok(_) => StepResult::NotFound,
            Err(e) => StepResult::Skipped {
                reason: format!("mDNS AAAA query failed: {e}"),
            },
        }
    }

    /// The real nss-mdns module never sends a packet for a name outside
    /// its configured suffix list (`.local` by default, via
    /// `/etc/mdns.allow` if present) — it returns UNAVAIL immediately,
    /// specifically so an ordinary internet domain can't get caught by
    /// its own `[NOTFOUND=return]` clause. Firing the probe unconditionally
    /// here (as gai did before this fix) meant a bare `NOT FOUND` where
    /// nss-mdns would've bailed out with UNAVAIL — misclassifying it as
    /// the exact NOTFOUND=return trap this tool exists to catch, on names
    /// that were never at risk of it. `/etc/mdns.allow`-style custom
    /// suffix lists aren't read here yet — MVP scope is the `.local`
    /// default, which covers the overwhelming majority of real configs.
    fn skip_non_local(name: &str) -> Option<String> {
        let n = name.trim_end_matches('.').to_ascii_lowercase();
        if n.ends_with(".local") || n == "local" {
            None
        } else {
            Some(format!(
                "'{name}' doesn't end in .local — nss-mdns wouldn't query mDNS for this name at all (real UNAVAIL, not a probed NOT FOUND)"
            ))
        }
    }
}

impl SourceResolver for SystemSourceResolver {
    fn resolve(&mut self, source: &NssSource, name: &str) -> StepResult {
        match source {
            NssSource::Files => self.lookup_hosts(name),
            NssSource::Dns => self.lookup_dns(name),
            NssSource::Mdns4Minimal | NssSource::Mdns4 => Self::lookup_mdns(name),
            NssSource::Mdns6Minimal | NssSource::Mdns6 => Self::lookup_mdns6(name),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skip_non_local_gates_ordinary_internet_domains() {
        // The bug this guards: google.com (or any non-.local name) must
        // never reach the network probe — it should be reported as
        // Skipped/Unavail, matching real nss-mdns, not a probed NotFound
        // that would trip the common [NOTFOUND=return] clause.
        assert!(SystemSourceResolver::skip_non_local("google.com").is_some());
        assert!(SystemSourceResolver::skip_non_local("example.com.").is_some());
    }

    #[test]
    fn skip_non_local_allows_dot_local_names() {
        assert!(SystemSourceResolver::skip_non_local("printer.local").is_none());
        assert!(SystemSourceResolver::skip_non_local("printer.local.").is_none());
        assert!(SystemSourceResolver::skip_non_local("PRINTER.LOCAL").is_none());
    }
}
