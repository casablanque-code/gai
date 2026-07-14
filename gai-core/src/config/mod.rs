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
    #[error("failed to read {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
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
