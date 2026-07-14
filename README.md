# gai — getaddrinfo inspector

`dig` works ≠ your process resolves. `gai` shows the actual decision path
the OS walks to turn a name into an IP — `/etc/hosts`, mDNS, systemd-resolved,
DNS, IPv6 policy — and where that path silently diverges from what a plain
DNS query would tell you.

No process interception. No LD_PRELOAD, no eBPF, no ptrace. `gai` reads the
same configuration the OS reads (`nsswitch.conf`, `resolv.conf`, `gai.conf`,
`/etc/hosts`, systemd-resolved's D-Bus state) and simulates the same
decision the OS would make — then cross-checks it against an independent
DNS query to catch the gap.

## Usage

```
gai explain api.local   # walk the resolution path, no verdict
gai doctor api.local    # walk the path + diagnose discrepancies
gai why api.local       # alias for doctor
```

## Install on a VPS

```
curl -fsSL https://raw.githubusercontent.com/casablanque-code/gai/main/install.sh | sudo bash
```

Pulls the latest static `x86_64-unknown-linux-musl` binary from
[Releases](../../releases) — no Rust toolchain needed on the target machine.

## Build from source

```
cargo build --release --package gai
```

## Workspace layout

- `gai-core` — pure parsing + simulation logic, no I/O, unit-tested against fixtures
- `gai-probe` — all real I/O: DNS queries, `/etc/hosts` reads, systemd-resolved D-Bus
- `gai` — CLI

## Scope (MVP)

Linux only. macOS (`scutil`/mDNSResponder) and Windows (DNS Client/LLMNR/NetBIOS)
are different enough architecturally that they're deliberately out of scope
for now rather than bolted on.
