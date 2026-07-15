use std::net::IpAddr;

/// One source that nsswitch can consult, in the order it appears in
/// `hosts:` line of /etc/nsswitch.conf.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NssSource {
    /// `files` — /etc/hosts.
    Files,
    /// `dns` — a real DNS query via the configured resolver.
    Dns,
    /// `mdns4_minimal` — one-shot IPv4 multicast DNS, the common
    /// desktop-default source that traps names ending in `.local`.
    Mdns4Minimal,
    /// `mdns6_minimal` — IPv6 counterpart of `mdns4_minimal`. Probing for
    /// this source isn't implemented yet (AAAA mDNS queries).
    Mdns6Minimal,
    /// `mdns4` — full (non-minimal) IPv4 mDNS source.
    Mdns4,
    /// `mdns6` — full (non-minimal) IPv6 mDNS source. Not yet probed.
    Mdns6,
    /// `myhostname` — systemd's synthetic source for the machine's own
    /// hostname and a handful of well-known names (`localhost`, etc).
    Myhostname,
    /// `resolve` — the systemd-resolved NSS module, distinct from asking
    /// resolv.conf's nameservers directly: it talks to resolved over
    /// D-Bus/socket and can answer even when resolv.conf just points at
    /// the 127.0.0.53 stub.
    Resolve,
    /// Any source nsswitch.conf lists that gai doesn't have specific
    /// handling for. Always resolves to `Skipped`, never guessed at.
    Other(String),
}

/// A `[STATUS=action]` clause attached to one NSS source, e.g. the
/// `[NOTFOUND=return]` in `mdns4_minimal [NOTFOUND=return]`. Determines
/// whether the chain halts or falls through to the next source when
/// that source's result matches `status`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NssCriterion {
    /// The result status this clause reacts to.
    pub status: NssStatus,
    /// What happens to the chain when `status` is matched.
    pub action: NssAction,
}

/// The classified outcome of trying one NSS source, used to match
/// against a source's `[STATUS=action]` criteria.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NssStatus {
    /// The source found an answer.
    Success,
    /// The source was reachable and definitively has no answer.
    NotFound,
    /// The source itself couldn't be consulted (service down, socket
    /// error, not configured) — different from a definitive NotFound.
    Unavail,
    /// The source is temporarily unable to answer (e.g. DNS timeout);
    /// distinct from a hard failure.
    TryAgain,
}

/// What happens to the nsswitch chain when a source's result matches one
/// of its criteria.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NssAction {
    /// Stop the chain here — later sources (including `dns`) are never
    /// tried. This is the action behind the classic mDNS trap.
    Return,
    /// Keep going to the next source even though this one matched.
    Continue,
}

/// One source entry from the `hosts:` line, with whatever criteria are
/// attached to it in nsswitch.conf.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NssEntry {
    /// Which NSS source this entry is.
    pub source: NssSource,
    /// The `[STATUS=action]` clauses attached to this source, if any. An
    /// empty vec means default glibc behavior applies (see
    /// [`crate::sim::simulate`]).
    pub criteria: Vec<NssCriterion>,
}

/// Parsed `/etc/nsswitch.conf`, currently only the `hosts:` line — the
/// rest of the file (passwd, group, shadow, etc.) isn't relevant to name
/// resolution and is ignored by the parser.
#[derive(Debug, Clone, Default)]
pub struct NsswitchConfig {
    /// The ordered list of sources from the `hosts:` line.
    pub hosts: Vec<NssEntry>,
}

/// Parsed `/etc/resolv.conf`.
#[derive(Debug, Clone, Default)]
pub struct ResolvConfig {
    /// Nameserver IPs from `nameserver` lines, in file order.
    pub nameservers: Vec<IpAddr>,
    /// Search domains from `search`/`domain` lines.
    pub search: Vec<String>,
    /// The `ndots` option — how many dots a name needs before it's tried
    /// as-is instead of with a search domain appended. glibc's default
    /// is 1 when the file doesn't set it explicitly.
    pub ndots: u32,
    /// True if this file is a systemd-resolved stub (127.0.0.53) rather
    /// than the real resolver config — a trap in itself.
    pub is_systemd_stub: bool,
}

/// Parsed `/etc/gai.conf` — IPv4/IPv6 address selection policy (RFC 6724),
/// the file almost nobody knows exists.
#[derive(Debug, Clone, Default)]
pub struct GaiConfig {
    /// `label` rules: (address prefix, label value).
    pub label_rules: Vec<(String, u32)>,
    /// `precedence` rules: (address prefix, precedence value).
    pub precedence_rules: Vec<(String, u32)>,
    /// Whether IPv6 addresses are preferred over IPv4 when both are
    /// available. True by default (glibc's compiled-in policy) even when
    /// no gai.conf file exists at all.
    pub prefer_ipv6: bool,
}

/// A single parsed line of `/etc/hosts` relevant to the queried name.
#[derive(Debug, Clone)]
pub struct HostsEntry {
    /// The address on this line.
    pub ip: IpAddr,
    /// All names this line maps to that address (a line can list several,
    /// e.g. `127.0.0.1 localhost localhost.localdomain`).
    pub names: Vec<String>,
}

/// How the target binary resolves names at all — not every process even
/// goes through glibc/NSS. Statically linked Go binaries are the classic
/// trap: they ship their own resolver and never touch nsswitch.conf.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolverRuntime {
    /// Dynamically linked against glibc — goes through the full NSS
    /// chain this crate simulates.
    GlibcNss,
    /// Dynamically linked against musl — also NSS-based, but musl's NSS
    /// implementation has its own quirks not modeled here yet.
    MuslNss,
    /// Statically linked Go binary using Go's built-in pure-Go resolver
    /// — bypasses nsswitch.conf entirely. Simulating the OS chain for
    /// such a binary would be simulating the wrong thing.
    GoPureResolver,
    /// Detection was inconclusive. Reported honestly rather than
    /// guessed at.
    Unknown,
}

/// One step in the simulated resolution path, in the order gai believes
/// the OS would actually walk it.
#[derive(Debug, Clone)]
pub struct ResolutionStep {
    /// Which NSS source this step tried.
    pub source: NssSource,
    /// What that source returned.
    pub result: StepResult,
    /// If this step halted the chain, the criterion that caused it
    /// (explicit `[STATUS=return]`, or `None` when it was the implicit
    /// default-SUCCESS halt with no criterion attached).
    pub halted_chain: Option<NssCriterion>,
}

/// The result of trying one NSS source during simulation.
#[derive(Debug, Clone)]
pub enum StepResult {
    /// The source found one or more addresses.
    Found(Vec<IpAddr>),
    /// The source was consulted and definitively has no answer.
    NotFound,
    /// The source couldn't be tried at all (not implemented yet, no
    /// nameservers configured, a query error). `reason` explains why —
    /// always surfaced to the user rather than silently treated as
    /// NotFound, since the two mean very different things for a
    /// diagnosis.
    Skipped { reason: String },
}
