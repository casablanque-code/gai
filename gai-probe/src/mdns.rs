use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, UdpSocket};
use std::time::Duration;

const MDNS_ADDR: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 251);
const MDNS_PORT: u16 = 5353;
const QUERY_TIMEOUT: Duration = Duration::from_millis(500);

const QTYPE_A: u16 = 1;
const QTYPE_AAAA: u16 = 28;

/// Sends a single one-shot mDNS A-record query for `name` and collects
/// whatever A records come back within QUERY_TIMEOUT. This is not a full
/// mDNS responder — no continuous listening, no cache, no service
/// discovery — just enough to answer "does anything on the local network
/// claim this name" the way NSS's mdns4_minimal source would.
pub fn query_a_record(name: &str) -> anyhow::Result<Vec<IpAddr>> {
    query_record(name, QTYPE_A)
}

/// Sends a single one-shot mDNS AAAA-record query for `name`, same
/// one-shot semantics as `query_a_record` above but for IPv6 — this is
/// what NSS's mdns6/mdns6_minimal sources do under the hood.
pub fn query_aaaa_record(name: &str) -> anyhow::Result<Vec<IpAddr>> {
    query_record(name, QTYPE_AAAA)
}

fn query_record(name: &str, qtype: u16) -> anyhow::Result<Vec<IpAddr>> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_read_timeout(Some(QUERY_TIMEOUT))?;

    let packet = build_query(name, qtype);
    socket.send_to(&packet, SocketAddr::from((MDNS_ADDR, MDNS_PORT)))?;

    let mut addresses = Vec::new();
    let mut buf = [0u8; 4096];
    let deadline = std::time::Instant::now() + QUERY_TIMEOUT;

    while std::time::Instant::now() < deadline {
        match socket.recv(&mut buf) {
            Ok(len) => {
                if let Some(found) = parse_records(&buf[..len], name, qtype) {
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
/// `name` with the given QTYPE (A=1 or AAAA=28), QCLASS=IN(1). Same wire
/// format as unicast DNS — mDNS just runs it over multicast UDP on port
/// 5353.
fn build_query(name: &str, qtype: u16) -> Vec<u8> {
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
    packet.extend_from_slice(&qtype.to_be_bytes());
    packet.extend_from_slice(&0x0001u16.to_be_bytes()); // QCLASS IN

    packet
}

/// Extracts A/AAAA-record addresses matching `qtype` from a response,
/// only if the question name in the response matches what we asked for
/// (best-effort — mDNS responders on the segment may answer other
/// queries too).
fn parse_records(data: &[u8], expected_name: &str, qtype: u16) -> Option<Vec<IpAddr>> {
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
        if rtype == qtype {
            let matches = decode_name(data, name_start)
                .map(|n| n.eq_ignore_ascii_case(expected_name))
                .unwrap_or(false);
            if matches {
                match (qtype, rdlength) {
                    (QTYPE_A, 4) => {
                        addrs.push(IpAddr::V4(Ipv4Addr::new(
                            data[pos],
                            data[pos + 1],
                            data[pos + 2],
                            data[pos + 3],
                        )));
                    }
                    (QTYPE_AAAA, 16) => {
                        let mut octets = [0u8; 16];
                        octets.copy_from_slice(&data[pos..pos + 16]);
                        addrs.push(IpAddr::V6(Ipv6Addr::from(octets)));
                    }
                    _ => {} // rtype matched qtype but rdlength is malformed — skip
                }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Appends a single A-record answer (with a name-compression pointer
    /// back to offset 12, where the question name starts) to `packet`,
    /// and returns the packet with header counts fixed up. This mirrors
    /// what a real mDNS responder puts on the wire well enough to drive
    /// `parse_records` through its real code path.
    fn append_a_answer(mut packet: Vec<u8>, ip: [u8; 4]) -> Vec<u8> {
        packet.extend_from_slice(&(0xC000u16 | 12).to_be_bytes()); // ptr to q-name
        packet.extend_from_slice(&QTYPE_A.to_be_bytes());
        packet.extend_from_slice(&1u16.to_be_bytes()); // CLASS IN
        packet.extend_from_slice(&0u32.to_be_bytes()); // TTL
        packet.extend_from_slice(&4u16.to_be_bytes()); // RDLENGTH
        packet.extend_from_slice(&ip);
        bump_ancount(packet)
    }

    fn append_aaaa_answer(mut packet: Vec<u8>, ip: [u8; 16]) -> Vec<u8> {
        packet.extend_from_slice(&(0xC000u16 | 12).to_be_bytes()); // ptr to q-name
        packet.extend_from_slice(&QTYPE_AAAA.to_be_bytes());
        packet.extend_from_slice(&1u16.to_be_bytes()); // CLASS IN
        packet.extend_from_slice(&0u32.to_be_bytes()); // TTL
        packet.extend_from_slice(&16u16.to_be_bytes()); // RDLENGTH
        packet.extend_from_slice(&ip);
        bump_ancount(packet)
    }

    fn bump_ancount(mut packet: Vec<u8>) -> Vec<u8> {
        let ancount = u16::from_be_bytes([packet[6], packet[7]]) + 1;
        packet[6..8].copy_from_slice(&ancount.to_be_bytes());
        packet
    }

    #[test]
    fn build_query_encodes_labels_and_root_and_qtype_a() {
        let packet = build_query("api.local", QTYPE_A);
        assert_eq!(&packet[0..12], &[0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0]);

        let expected_question = b"\x03api\x05local\x00";
        assert_eq!(&packet[12..12 + expected_question.len()], expected_question);
        let after = 12 + expected_question.len();
        assert_eq!(&packet[after..after + 2], &QTYPE_A.to_be_bytes());
        assert_eq!(&packet[after + 2..after + 4], &1u16.to_be_bytes());
    }

    #[test]
    fn build_query_uses_qtype_aaaa_when_requested() {
        let packet = build_query("api.local", QTYPE_AAAA);
        let expected_question = b"\x03api\x05local\x00";
        let after = 12 + expected_question.len();
        assert_eq!(&packet[after..after + 2], &QTYPE_AAAA.to_be_bytes());
    }

    #[test]
    fn build_query_skips_empty_labels_from_trailing_dot() {
        let packet = build_query("api.local.", QTYPE_A);
        let expected_question = b"\x03api\x05local\x00";
        assert_eq!(&packet[12..12 + expected_question.len()], expected_question);
    }

    #[test]
    fn parse_records_extracts_matching_a_answer() {
        let query = build_query("api.local", QTYPE_A);
        let packet = append_a_answer(query, [192, 168, 1, 42]);

        let addrs = parse_records(&packet, "api.local", QTYPE_A).expect("should find a record");
        assert_eq!(addrs, vec![IpAddr::V4(Ipv4Addr::new(192, 168, 1, 42))]);
    }

    #[test]
    fn parse_records_extracts_matching_aaaa_answer() {
        let query = build_query("api.local", QTYPE_AAAA);
        let ip = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1);
        let packet = append_aaaa_answer(query, ip.octets());

        let addrs =
            parse_records(&packet, "api.local", QTYPE_AAAA).expect("should find an AAAA record");
        assert_eq!(addrs, vec![IpAddr::V6(ip)]);
    }

    #[test]
    fn parse_records_ignores_aaaa_answer_when_asking_for_a() {
        // A responder that only has an AAAA record must not be reported
        // as an answer to an A-type query, even though the name matches.
        let query = build_query("api.local", QTYPE_A);
        let ip = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1);
        let packet = append_aaaa_answer(query, ip.octets());

        assert!(parse_records(&packet, "api.local", QTYPE_A).is_none());
    }

    #[test]
    fn parse_records_ignores_answer_for_a_different_name() {
        let query = build_query("printer.local", QTYPE_A);
        let packet = append_a_answer(query, [10, 0, 0, 5]);

        let addrs = parse_records(&packet, "api.local", QTYPE_A);
        assert!(
            addrs.is_none(),
            "must not report a match for the wrong name"
        );
    }

    #[test]
    fn parse_records_returns_none_with_zero_answers() {
        let query = build_query("api.local", QTYPE_A); // ancount stays 0
        assert!(parse_records(&query, "api.local", QTYPE_A).is_none());
    }

    #[test]
    fn parse_records_returns_none_on_truncated_packet() {
        let mut packet = append_a_answer(build_query("api.local", QTYPE_A), [1, 2, 3, 4]);
        packet.truncate(packet.len() - 3); // cut off mid-rdata
        assert!(parse_records(&packet, "api.local", QTYPE_A).is_none());
    }

    #[test]
    fn skip_name_follows_compression_pointer_as_two_bytes() {
        let data = [0xC0, 0x00, 0xAA];
        let next = skip_name(&data, 0).expect("valid pointer");
        assert_eq!(next, 2);
    }

    #[test]
    fn skip_name_returns_none_on_truncated_label() {
        let data = [5u8, b'a', b'b'];
        assert!(skip_name(&data, 0).is_none());
    }

    #[test]
    fn decode_name_reassembles_dotted_labels() {
        let mut data = vec![0u8; 12];
        data.extend_from_slice(b"\x03api\x05local\x00");
        let name = decode_name(&data, 12).expect("decodes");
        assert_eq!(name, "api.local");
    }

    #[test]
    fn decode_name_guards_against_pointer_loops() {
        let data = [0xC0, 0x00, 0xC0, 0x00];
        assert!(decode_name(&data, 0).is_none());
    }
}
