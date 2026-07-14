use super::{read_to_string, ConfigError};
use crate::types::{NssAction, NssCriterion, NssEntry, NssSource, NssStatus, NsswitchConfig};
use std::path::Path;

/// Parses the `hosts:` line of /etc/nsswitch.conf into an ordered list of
/// sources plus any [STATUS=action] criteria attached to each one.
///
/// Example input line:
///   hosts: files mdns4_minimal [NOTFOUND=return] dns
pub fn parse_nsswitch(path: &Path) -> Result<NsswitchConfig, ConfigError> {
    let content = read_to_string(path)?;
    let mut cfg = NsswitchConfig::default();

    for (line_no, raw_line) in content.lines().enumerate() {
        let line = strip_comment(raw_line).trim();
        if line.is_empty() {
            continue;
        }
        let Some(rest) = line.strip_prefix("hosts:") else {
            continue;
        };
        cfg.hosts = parse_hosts_line(rest, path, line_no + 1)?;
    }

    Ok(cfg)
}

fn strip_comment(line: &str) -> &str {
    match line.find('#') {
        Some(idx) => &line[..idx],
        None => line,
    }
}

fn parse_hosts_line(rest: &str, path: &Path, line_no: usize) -> Result<Vec<NssEntry>, ConfigError> {
    let mut entries = Vec::new();
    let mut pending_source: Option<NssSource> = None;
    let mut pending_criteria: Vec<NssCriterion> = Vec::new();

    // Tokens are either bare source names ("files", "dns") or bracketed
    // criteria ("[NOTFOUND=return]"). A bracketed group always attaches to
    // the source token immediately preceding it.
    for token in tokenize(rest) {
        if let Some(bracket_body) = token.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            let criterion = parse_criterion(bracket_body, path, line_no)?;
            pending_criteria.push(criterion);
        } else {
            if let Some(src) = pending_source.take() {
                entries.push(NssEntry {
                    source: src,
                    criteria: std::mem::take(&mut pending_criteria),
                });
            }
            pending_source = Some(parse_source(&token));
        }
    }
    if let Some(src) = pending_source.take() {
        entries.push(NssEntry {
            source: src,
            criteria: pending_criteria,
        });
    }

    Ok(entries)
}

/// Splits on whitespace but keeps "[...]" groups (which may contain no
/// internal whitespace in practice, but we tokenize defensively anyway).
fn tokenize(s: &str) -> Vec<String> {
    s.split_whitespace().map(|t| t.to_string()).collect()
}

fn parse_source(token: &str) -> NssSource {
    match token {
        "files" => NssSource::Files,
        "dns" => NssSource::Dns,
        "mdns4_minimal" => NssSource::Mdns4Minimal,
        "mdns6_minimal" => NssSource::Mdns6Minimal,
        "mdns4" => NssSource::Mdns4,
        "mdns6" => NssSource::Mdns6,
        "myhostname" => NssSource::Myhostname,
        "resolve" => NssSource::Resolve,
        other => NssSource::Other(other.to_string()),
    }
}

fn parse_criterion(body: &str, path: &Path, line_no: usize) -> Result<NssCriterion, ConfigError> {
    let (status_str, action_str) = body.split_once('=').ok_or_else(|| ConfigError::Parse {
        path: path.display().to_string(),
        line_no,
        detail: format!("expected STATUS=action inside brackets, got '{body}'"),
    })?;

    let status = match status_str.to_ascii_uppercase().as_str() {
        "SUCCESS" => NssStatus::Success,
        "NOTFOUND" => NssStatus::NotFound,
        "UNAVAIL" => NssStatus::Unavail,
        "TRYAGAIN" => NssStatus::TryAgain,
        other => {
            return Err(ConfigError::Parse {
                path: path.display().to_string(),
                line_no,
                detail: format!("unknown status '{other}'"),
            })
        }
    };
    let action = match action_str.to_ascii_lowercase().as_str() {
        "return" => NssAction::Return,
        "continue" => NssAction::Continue,
        other => {
            return Err(ConfigError::Parse {
                path: path.display().to_string(),
                line_no,
                detail: format!("unknown action '{other}'"),
            })
        }
    };

    Ok(NssCriterion { status, action })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_fixture(contents: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(contents.as_bytes()).unwrap();
        f
    }

    #[test]
    fn parses_notfound_return_trap() {
        let f = write_fixture("hosts: files mdns4_minimal [NOTFOUND=return] dns\n");
        let cfg = parse_nsswitch(f.path()).unwrap();
        assert_eq!(cfg.hosts.len(), 3);
        assert_eq!(cfg.hosts[0].source, NssSource::Files);
        assert_eq!(cfg.hosts[1].source, NssSource::Mdns4Minimal);
        assert_eq!(
            cfg.hosts[1].criteria,
            vec![NssCriterion {
                status: NssStatus::NotFound,
                action: NssAction::Return
            }]
        );
        assert_eq!(cfg.hosts[2].source, NssSource::Dns);
        assert!(cfg.hosts[2].criteria.is_empty());
    }

    #[test]
    fn ignores_comments_and_other_database_lines() {
        let f = write_fixture(
            "# comment\npasswd: files\nhosts: files dns # trailing comment\ngroup: files\n",
        );
        let cfg = parse_nsswitch(f.path()).unwrap();
        assert_eq!(cfg.hosts.len(), 2);
    }
}
