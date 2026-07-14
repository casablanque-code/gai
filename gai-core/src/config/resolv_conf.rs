use super::{read_to_string, ConfigError};
use crate::types::ResolvConfig;
use std::net::IpAddr;
use std::path::Path;

/// Parses /etc/resolv.conf.
///
/// Critically: on most modern distros this file just points at
/// 127.0.0.53 (the systemd-resolved stub listener). Reporting that IP
/// as "the DNS server" would be a lie — the real per-link servers live
/// in resolved's runtime state, not in this file. We flag that case so
/// callers know to go ask gai-probe's resolved client instead of trusting
/// this file at face value.
pub fn parse_resolv_conf(path: &Path) -> Result<ResolvConfig, ConfigError> {
    let content = read_to_string(path)?;
    let mut cfg = ResolvConfig {
        ndots: 1, // glibc default
        ..Default::default()
    };

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let Some(keyword) = parts.next() else {
            continue;
        };
        match keyword {
            "nameserver" => {
                if let Some(addr) = parts.next().and_then(|s| s.parse::<IpAddr>().ok()) {
                    cfg.nameservers.push(addr);
                }
            }
            "search" | "domain" => {
                cfg.search.extend(parts.map(|s| s.to_string()));
            }
            "options" => {
                for opt in parts {
                    if let Some(n) = opt.strip_prefix("ndots:") {
                        cfg.ndots = n.parse().unwrap_or(1);
                    }
                }
            }
            _ => {}
        }
    }

    cfg.is_systemd_stub = cfg
        .nameservers
        .iter()
        .any(|ip| matches!(ip, IpAddr::V4(v4) if *v4 == std::net::Ipv4Addr::new(127, 0, 0, 53)));

    Ok(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn detects_systemd_resolved_stub() {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        writeln!(f, "nameserver 127.0.0.53\noptions edns0 trust-ad").unwrap();
        let cfg = parse_resolv_conf(f.path()).unwrap();
        assert!(cfg.is_systemd_stub);
    }

    #[test]
    fn parses_real_nameservers_and_ndots() {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        writeln!(
            f,
            "nameserver 8.8.8.8\nnameserver 1.1.1.1\nsearch corp.internal\noptions ndots:2"
        )
        .unwrap();
        let cfg = parse_resolv_conf(f.path()).unwrap();
        assert_eq!(cfg.nameservers.len(), 2);
        assert_eq!(cfg.search, vec!["corp.internal"]);
        assert_eq!(cfg.ndots, 2);
        assert!(!cfg.is_systemd_stub);
    }
}
