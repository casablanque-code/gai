use super::{read_to_string, ConfigError};
use crate::types::GaiConfig;
use std::path::Path;

/// Parses /etc/gai.conf — RFC 6724 address selection policy.
/// Most systems have no such file, meaning glibc's compiled-in defaults
/// apply (which themselves prefer IPv6). Absence of the file is a valid,
/// meaningful result, not an error.
pub fn parse_gai_conf(path: &Path) -> Result<GaiConfig, ConfigError> {
    if !path.exists() {
        return Ok(GaiConfig {
            prefer_ipv6: true, // glibc built-in default policy
            ..Default::default()
        });
    }

    let content = read_to_string(path)?;
    let mut cfg = GaiConfig {
        prefer_ipv6: true,
        ..Default::default()
    };

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        match parts.next() {
            Some("label") => {
                if let (Some(prefix), Some(val)) = (parts.next(), parts.next()) {
                    if let Ok(v) = val.parse::<u32>() {
                        cfg.label_rules.push((prefix.to_string(), v));
                    }
                }
            }
            Some("precedence") => {
                if let (Some(prefix), Some(val)) = (parts.next(), parts.next()) {
                    if let Ok(v) = val.parse::<u32>() {
                        cfg.precedence_rules.push((prefix.to_string(), v));
                    }
                }
            }
            _ => {}
        }
    }

    Ok(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_file_uses_glibc_default_prefer_ipv6() {
        let cfg = parse_gai_conf(Path::new("/nonexistent/gai.conf")).unwrap();
        assert!(cfg.prefer_ipv6);
        assert!(cfg.label_rules.is_empty());
        assert!(cfg.precedence_rules.is_empty());
    }

    #[test]
    fn parses_label_and_precedence_rules() {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        use std::io::Write;
        writeln!(f, "# comment").unwrap();
        writeln!(f, "label ::1/128 0").unwrap();
        writeln!(f, "precedence ::ffff:0:0/96 100").unwrap();
        let cfg = parse_gai_conf(f.path()).unwrap();

        assert_eq!(cfg.label_rules, vec![("::1/128".to_string(), 0)]);
        assert_eq!(
            cfg.precedence_rules,
            vec![("::ffff:0:0/96".to_string(), 100)]
        );
    }
}
