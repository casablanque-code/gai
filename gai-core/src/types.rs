use std::net::IpAddr;

/// One source that nsswitch can consult, in the order it appears in
/// `hosts:` line of /etc/nsswitch.conf.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NssSource {
    Files,
    Dns,
    Mdns4Minimal,
    Mdns6Minimal,
    Mdns4,
    Mdns6,
    Myhostname,
    Resolve, // systemd-resolved NSS module
    Other(String),
}

/// An action that terminates the nsswitch chain early, e.g. [NOTFOUND=return]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NssCriterion {
    pub status: NssStatus,
    pub action: NssAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NssStatus {
    Success,
    NotFound,
    Unavail,
    TryAgain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NssAction {
    Return,
    Continue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NssEntry {
    pub source: NssSource,
    pub criteria: Vec<NssCriterion>,
}

#[derive(Debug, Clone, Default)]
pub struct NsswitchConfig {
    pub hosts: Vec<NssEntry>,
}

/// Parsed /etc/resolv.conf
#[derive(Debug, Clone, Default)]
pub struct ResolvConfig {
    pub nameservers: Vec<IpAddr>,
    pub search: Vec<String>,
    pub ndots: u32,
    /// True if this file is a systemd-resolved stub (127.0.0.53) rather
    /// than the real resolver config — a trap in itself.
    pub is_systemd_stub: bool,
}

/// Parsed /etc/gai.conf — IPv4/IPv6 address selection policy (RFC 6724),
/// the file almost nobody knows exists.
#[derive(Debug, Clone, Default)]
pub struct GaiConfig {
    pub label_rules: Vec<(String, u32)>,
    pub precedence_rules: Vec<(String, u32)>,
    pub prefer_ipv6: bool,
}

/// A single parsed line of /etc/hosts relevant to the queried name.
#[derive(Debug, Clone)]
pub struct HostsEntry {
    pub ip: IpAddr,
    pub names: Vec<String>,
}

/// How the target binary resolves names at all — not every process even
/// goes through glibc/NSS. Statically linked Go binaries are the classic
/// trap: they ship their own resolver and never touch nsswitch.conf.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolverRuntime {
    GlibcNss,
    MuslNss,
    GoPureResolver,
    Unknown,
}

/// One step in the simulated resolution path, in the order gai believes
/// the OS would actually walk it.
#[derive(Debug, Clone)]
pub struct ResolutionStep {
    pub source: NssSource,
    pub result: StepResult,
    pub halted_chain: Option<NssCriterion>,
}

#[derive(Debug, Clone)]
pub enum StepResult {
    Found(Vec<IpAddr>),
    NotFound,
    Skipped { reason: String },
}
