use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use zbus::blocking::Connection;

/// One entry of org.freedesktop.resolve1.Manager's "DNS" property:
/// (ifindex, family, address_bytes).
type RawDnsEntry = (i32, i32, Vec<u8>);

#[derive(Debug, Clone)]
pub struct ResolvedNameserver {
    pub ifindex: i32,
    pub address: IpAddr,
}

/// Queries systemd-resolved over the system D-Bus for the nameservers it
/// is actually using — the ones hidden behind the 127.0.0.53 stub that
/// resolv.conf shows instead. This is a config/state read, not process
/// interception: same principle as parsing nsswitch.conf, just over
/// D-Bus instead of a file.
///
/// Returns an empty vec (not an error) if resolved isn't running, since
/// that's a perfectly normal system state, not a failure.
pub fn query_nameservers() -> anyhow::Result<Vec<ResolvedNameserver>> {
    let connection = match Connection::system() {
        Ok(c) => c,
        Err(_) => return Ok(Vec::new()),
    };

    let proxy = zbus::blocking::Proxy::new(
        &connection,
        "org.freedesktop.resolve1",
        "/org/freedesktop/resolve1",
        "org.freedesktop.resolve1.Manager",
    )?;

    let raw: Vec<RawDnsEntry> = match proxy.get_property("DNS") {
        Ok(v) => v,
        // resolved not running / property unavailable — not an error state.
        Err(_) => return Ok(Vec::new()),
    };

    Ok(raw
        .into_iter()
        .filter_map(|(ifindex, family, bytes)| {
            let address = decode_address(family, &bytes)?;
            Some(ResolvedNameserver { ifindex, address })
        })
        .collect())
}

fn decode_address(family: i32, bytes: &[u8]) -> Option<IpAddr> {
    const AF_INET: i32 = 2;
    const AF_INET6: i32 = 10;
    match family {
        AF_INET if bytes.len() == 4 => Some(IpAddr::V4(Ipv4Addr::new(
            bytes[0], bytes[1], bytes[2], bytes[3],
        ))),
        AF_INET6 if bytes.len() == 16 => {
            let mut octets = [0u8; 16];
            octets.copy_from_slice(bytes);
            Some(IpAddr::V6(Ipv6Addr::from(octets)))
        }
        _ => None,
    }
}

/// Convenience: flat list of unique addresses, ignoring which link they
/// came from. Kept for callers that genuinely want every configured
/// server regardless of scope; general-purpose queries should use
/// [`effective_addresses`] instead — see its doc comment for why.
pub fn flat_addresses(entries: &[ResolvedNameserver]) -> Vec<IpAddr> {
    let mut addrs: Vec<IpAddr> = entries.iter().map(|e| e.address).collect();
    addrs.sort();
    addrs.dedup();
    addrs
}

/// Nameservers to use for an ordinary, unscoped name query (the plain
/// `dns` NSS source, and the reality check).
///
/// systemd-resolved's Manager.DNS property mixes two different kinds of
/// entry: global-scope servers (ifindex 0, set via `resolv.conf`/global
/// `DNS=`) meant for any name, and per-link servers meant only for names
/// under that link's routing domains (e.g. a VPN's split-DNS resolver,
/// which answers its own private zone and SERVFAILs — not NXDOMAINs —
/// anything else). `flat_addresses` throws that distinction away and
/// pools both kinds together, so a link-scoped resolver ends up being
/// asked about public internet names it was never meant to answer.
///
/// This is a partial fix, not full split-DNS: it doesn't route a name to
/// the specific link whose domain it matches (that needs the Domains
/// property too, and is a real follow-up — see the split-DNS gap noted
/// elsewhere). It does stop link-scoped-only resolvers from being queried
/// for names outside their scope by default, which is the failure mode
/// this function exists to avoid: prefer global-scope servers, and only
/// fall back to the full unscoped pool if no global-scope server was
/// reported at all (e.g. a system with only per-link DNS configured).
pub fn effective_addresses(entries: &[ResolvedNameserver]) -> Vec<IpAddr> {
    let global: Vec<IpAddr> = entries
        .iter()
        .filter(|e| e.ifindex == 0)
        .map(|e| e.address)
        .collect();
    let mut addrs = if global.is_empty() {
        entries.iter().map(|e| e.address).collect()
    } else {
        global
    };
    addrs.sort();
    addrs.dedup();
    addrs
}
