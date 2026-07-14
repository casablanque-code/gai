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
