use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::time::Duration;

const MDNS_ADDR: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 251);
const MDNS_PORT: u16 = 5353;
const QUERY_TIMEOUT: Duration = Duration::from_millis(500);

/// Sends a single one-shot mDNS A-record query for `name` and collects
/// whatever A records come back within QUERY_TIMEOUT. This is not a full
/// mDNS responder — no continuous listening, no cache, no service
/// discovery — just enough to answer "does anything on the local network
/// claim this name" the way NSS's mdns4_minimal source would.
pub fn query_a_record(name: &str) -> anyhow::Result<Vec<IpAddr>> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_read_timeout(Some(QUERY_TIMEOUT))?;

    let packet = build_query(name);
    socket.send_to(&packet, SocketAddr::from((MDNS_ADDR, MDNS_PORT)))?;

    let mut addresses = Vec::new();
    let mut buf = [0u8; 4096];
    let deadline = std::time::Instant::now() + QUERY_TIMEOUT;

    while std::time::Instant::now() < deadline {
        match socket.recv(&mut buf) {
            Ok(len) => {
                if let Some(found) = parse_a_records(&buf[..len], name) {
                    addresses.extend(found);
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => break,
            Err(_) => break,
        }
    }

    addresses.sort();
    addresses.dedup();
    Ok(addresses)
}

/// Builds a minimal DNS query packet: 12-byte header + one question for
/// `name` with QTYPE=A(1), QCLASS=IN(1). Same wire format as unicast DNS —
/// mDNS just runs it over multicast UDP on port 5353.
fn build_query(name: &str) -> Vec<u8> {
    let mut packet = Vec::with_capacity(32);
    packet.extend_from_slice(&0x0000u16.to_be_bytes()); // transaction ID (0 for mDNS)
    packet.extend_from_slice(&0x0000u16.to_be_bytes()); // flags: standard query
    packet.extend_from_slice(&0x0001u16.to_be_bytes()); // qdcount = 1
    packet.extend_from_slice(&0x0000u16.to_be_bytes()); // ancount
    packet.extend_from_slice(&0x0000u16.to_be_bytes()); // nscount
    packet.extend_from_slice(&0x0000u16.to_be_bytes()); // arcount

    for label in name.split('.') {
        if label.is_empty() {
            continue;
        }
        packet.push(label.len() as u8);
        packet.extend_from_slice(label.as_bytes());
    }
    packet.push(0x00); // root label
    packet.extend_from_slice(&0x0001u16.to_be_bytes()); // QTYPE A
    packet.extend_from_slice(&0x0001u16.to_be_bytes()); // QCLASS IN

    packet
}

/// Extracts A-record addresses from a response, only if the question
/// name in the response matches what we asked for (best-effort — mDNS
/// responders on the segment may answer other queries too).
fn parse_a_records(data: &[u8], expected_name: &str) -> Option<Vec<IpAddr>> {
    if data.len() < 12 {
        return None;
    }
    let ancount = u16::from_be_bytes([data[6], data[7]]) as usize;
    if ancount == 0 {
        return None;
    }

    let mut pos = 12;
    // Skip questions section (qdcount).
    let qdcount = u16::from_be_bytes([data[4], data[5]]) as usize;
    for _ in 0..qdcount {
        pos = skip_name(data, pos)?;
        pos += 4; // QTYPE + QCLASS
    }

    let mut addrs = Vec::new();
    for _ in 0..ancount {
        let name_start = pos;
        pos = skip_name(data, pos)?;
        if pos + 10 > data.len() {
            break;
        }
        let rtype = u16::from_be_bytes([data[pos], data[pos + 1]]);
        let rdlength = u16::from_be_bytes([data[pos + 8], data[pos + 9]]) as usize;
        pos += 10;
        if pos + rdlength > data.len() {
            break;
        }
        if rtype == 1 && rdlength == 4 {
            let matches = decode_name(data, name_start)
                .map(|n| n.eq_ignore_ascii_case(expected_name))
                .unwrap_or(false);
            if matches {
                addrs.push(IpAddr::V4(Ipv4Addr::new(
                    data[pos],
                    data[pos + 1],
                    data[pos + 2],
                    data[pos + 3],
                )));
            }
        }
        pos += rdlength;
    }

    if addrs.is_empty() {
        None
    } else {
        Some(addrs)
    }
}

/// Advances past a (possibly compressed) DNS name and returns the new
/// offset, without needing the decoded value.
fn skip_name(data: &[u8], mut pos: usize) -> Option<usize> {
    loop {
        let len = *data.get(pos)? as usize;
        if len == 0 {
            return Some(pos + 1);
        }
        if len & 0xC0 == 0xC0 {
            // compression pointer: 2 bytes, doesn't recurse for skip purposes
            return Some(pos + 2);
        }
        pos += 1 + len;
        if pos >= data.len() {
            return None;
        }
    }
}

/// Decodes a (possibly compressed) DNS name starting at `pos` into a
/// dotted string, following at most one compression pointer hop.
fn decode_name(data: &[u8], mut pos: usize) -> Option<String> {
    let mut labels = Vec::new();
    let mut hops = 0;
    loop {
        let len = *data.get(pos)? as usize;
        if len == 0 {
            break;
        }
        if len & 0xC0 == 0xC0 {
            if hops > 5 {
                return None; // guard against pointer loops
            }
            hops += 1;
            let next = ((len & 0x3F) << 8) | (*data.get(pos + 1)? as usize);
            pos = next;
            continue;
        }
        let start = pos + 1;
        let end = start + len;
        labels.push(std::str::from_utf8(data.get(start..end)?).ok()?.to_string());
        pos = end;
    }
    Some(labels.join("."))
}
