mod gai_conf;
mod hosts;
mod nsswitch;
mod resolv_conf;

pub use gai_conf::parse_gai_conf;
pub use hosts::parse_hosts;
pub use nsswitch::parse_nsswitch;
pub use resolv_conf::parse_resolv_conf;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    /// The file couldn't be read at all (missing, permissions, etc).
    /// Note: absent `/etc/gai.conf` is *not* an error — see
    /// [`crate::config::parse_gai_conf`], which treats that case as a
    /// meaningful default rather than a failure.
    #[error("failed to read {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    /// The file was read but a line didn't parse — e.g. an unrecognized
    /// `[STATUS=action]` clause in nsswitch.conf.
    #[error("malformed line {line_no} in {path}: {detail}")]
    Parse {
        path: String,
        line_no: usize,
        detail: String,
    },
}

pub(crate) fn read_to_string(path: &Path) -> Result<String, ConfigError> {
    std::fs::read_to_string(path).map_err(|source| ConfigError::Io {
        path: path.display().to_string(),
        source,
    })
}
