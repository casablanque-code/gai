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

pub fn detect() -> Platform {
    Platform::Linux
}

pub mod paths {
    pub const NSSWITCH_CONF: &str = "/etc/nsswitch.conf";
    pub const RESOLV_CONF: &str = "/etc/resolv.conf";
    pub const GAI_CONF: &str = "/etc/gai.conf";
    pub const HOSTS: &str = "/etc/hosts";
}
