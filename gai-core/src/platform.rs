/// Which OS-level name resolution model we're simulating.
///
/// MVP scope is Linux only. macOS resolution goes through mDNSResponder /
/// `scutil --dns` and largely ignores resolv.conf — different enough that
/// it needs its own module later, not a branch bolted onto this one.
/// Windows (DNS Client service, LLMNR, NetBIOS) is further out still.
/// Deliberately not building either now — see manifesto: no cathedral.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Linux,
}

/// Currently always returns [`Platform::Linux`] — see [`Platform`] for why
/// the other variants don't exist yet.
pub fn detect() -> Platform {
    Platform::Linux
}

/// Well-known config file paths this crate's parsers read from.
pub mod paths {
    /// The `hosts:` line here drives the whole simulation.
    pub const NSSWITCH_CONF: &str = "/etc/nsswitch.conf";
    /// May be a symlink to systemd-resolved's stub — see
    /// [`crate::types::ResolvConfig::is_systemd_stub`].
    pub const RESOLV_CONF: &str = "/etc/resolv.conf";
    /// RFC 6724 address-selection policy. Absence is meaningful, not an
    /// error — see [`crate::config::parse_gai_conf`].
    pub const GAI_CONF: &str = "/etc/gai.conf";
    /// Consulted by the `files` NSS source.
    pub const HOSTS: &str = "/etc/hosts";
}
