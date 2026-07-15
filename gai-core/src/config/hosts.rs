use super::{read_to_string, ConfigError};
use crate::types::HostsEntry;
use std::net::IpAddr;
use std::path::Path;

/// Parses /etc/hosts into entries. Callers filter by queried name.
pub fn parse_hosts(path: &Path) -> Result<Vec<HostsEntry>, ConfigError> {
    let content = read_to_string(path)?;
    let mut entries = Vec::new();

    for line in content.lines() {
        let line = match line.find('#') {
            Some(idx) => &line[..idx],
            None => line,
        }
        .trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.split_whitespace();
        let Some(ip_str) = parts.next() else { continue };
        let Ok(ip) = ip_str.parse::<IpAddr>() else {
            continue;
        };
        let names: Vec<String> = parts.map(|s| s.to_string()).collect();
        if names.is_empty() {
            continue;
        }
        entries.push(HostsEntry { ip, names });
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parses_multiple_names_per_ip() {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        writeln!(f, "127.0.0.1 localhost localhost.localdomain").unwrap();
        writeln!(f, "10.0.0.1 testhost.local testhost").unwrap();
        let entries = parse_hosts(f.path()).unwrap();

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[1].ip, "10.0.0.1".parse::<IpAddr>().unwrap());
        assert_eq!(entries[1].names, vec!["testhost.local", "testhost"]);
    }

    #[test]
    fn ignores_comments_and_blank_lines() {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        writeln!(f, "# comment line").unwrap();
        writeln!(f).unwrap();
        writeln!(f, "10.0.0.2 realhost # trailing comment").unwrap();
        let entries = parse_hosts(f.path()).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].names, vec!["realhost"]);
    }

    #[test]
    fn skips_malformed_lines_without_erroring() {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        writeln!(f, "not-an-ip somehost").unwrap();
        writeln!(f, "10.0.0.3 validhost").unwrap();
        let entries = parse_hosts(f.path()).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].names, vec!["validhost"]);
    }

    #[test]
    fn parses_ipv6_entries() {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        writeln!(f, "::1 ip6-localhost ip6-loopback").unwrap();
        let entries = parse_hosts(f.path()).unwrap();

        assert_eq!(entries[0].ip, "::1".parse::<IpAddr>().unwrap());
    }
}
