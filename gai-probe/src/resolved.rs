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
        AF_INET if bytes.len() == 4 => {
            Some(IpAddr::V4(Ipv4Addr::new(bytes[0], bytes[1], bytes[2], bytes[3])))
        }
        AF_INET6 if bytes.len() == 16 => {
            let mut octets = [0u8; 16];
            octets.copy_from_slice(bytes);
            Some(IpAddr::V6(Ipv6Addr::from(octets)))
        }
        _ => None,
    }
}

/// Convenience: flat list of unique addresses, ignoring which link they
/// came from. Good enough for the reality-check query; per-link
/// split-DNS accuracy is a follow-up, not MVP.
pub fn flat_addresses(entries: &[ResolvedNameserver]) -> Vec<IpAddr> {
    let mut addrs: Vec<IpAddr> = entries.iter().map(|e| e.address).collect();
    addrs.sort();
    addrs.dedup();
    addrs
}
