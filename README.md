# gai — getaddrinfo inspector

`dig` works ≠ your process resolves. `gai` shows the actual decision path
the OS walks to turn a name into an IP — `/etc/hosts`, mDNS, systemd-resolved,
DNS — and cross-checks it against an independent DNS query to catch where
that path silently diverges.

No process interception. No LD_PRELOAD, no eBPF, no ptrace. `gai` reads the
same configuration the OS reads (`nsswitch.conf`, `resolv.conf`, `gai.conf`,
`/etc/hosts`, systemd-resolved's D-Bus state, one-shot mDNS) and simulates
the same decision the OS would make.

## Example

```
$ gai doctor testhost.local
[gai] Simulating name resolution for "testhost.local"...
  (reality check via [212.227.123.16, 212.227.123.17], systemd-resolved stub: true)
RESOLUTION PATH (simulated):
  1. [Files] Found([10.0.0.1])
DIAGNOSIS:
  The OS chain and a direct DNS query disagree: [10.0.0.1] vs []. Something
  earlier in the chain (files/mdns) is answering instead of DNS.
```

`testhost.local` was added to `/etc/hosts`. The chain halted there — DNS was
never asked. A direct, independent DNS query (against systemd-resolved's
*real* per-link nameservers, not the `127.0.0.53` stub) confirms it: nobody
out there has heard of this name. `gai` shows both sides instead of leaving
you to guess which one lied.

## Usage

```
gai explain <name>   # walk the resolution path, no verdict
gai doctor <name>    # walk the path + diagnose discrepancies
gai why <name>       # alias for doctor
```

## Install on a VPS (static binary, no Rust toolchain needed)

```
curl -fsSL https://raw.githubusercontent.com/casablanque-code/gai/main/install.sh | sudo bash
```

Pulls the latest `x86_64-unknown-linux-musl` binary from
[Releases](../../releases). Pin a version with
`... | sudo bash -s -- v0.1.3`.

## Install via cargo

```
cargo install gai-inspector   # installs a binary named `gai`
```

## Build from source

```
cargo build --release --package gai-inspector
```

## What it actually does

- **`/etc/nsswitch.conf`**: parses the `hosts:` line and walks it exactly
  like glibc's NSS dispatcher — including `[STATUS=action]` criteria such
  as `[NOTFOUND=return]`, the rule that silently stops a name from ever
  reaching DNS.
- **`/etc/resolv.conf`**: detects the systemd-resolved stub (`127.0.0.53`)
  instead of reporting it as "the DNS server" — because it isn't one.
- **systemd-resolved (D-Bus)**: when the stub is in play, queries
  `org.freedesktop.resolve1.Manager`'s `DNS` property for the real
  per-link nameservers hiding behind it.
- **mDNS**: a genuine one-shot A-record query on `224.0.0.251:5353` for
  the `mdns4_minimal`/`mdns4` NSS sources (IPv6/AAAA not yet implemented).
- **Reality check**: an independent DNS query — with `/etc/hosts` lookup
  explicitly disabled on the resolver — so it can never spuriously agree
  with a Files-based result just because both happened to read the same
  file.
- **Resolver runtime detection** (best-effort): flags statically linked Go
  binaries, which ship their own pure-Go resolver and bypass NSS entirely —
  simulating `nsswitch.conf` for such a binary would be simulating the
  wrong thing.

## Workspace layout

| Crate | Published as | What it is |
|---|---|---|
| `gai-core` | [`gai-core`](https://crates.io/crates/gai-core) | Pure parsing + simulation logic, no I/O, unit-tested against fixtures |
| `gai-probe` | [`gai-probe`](https://crates.io/crates/gai-probe) | All real I/O: DNS queries, `/etc/hosts` reads, systemd-resolved D-Bus, mDNS |
| `gai` (package `gai-inspector`) | [`gai-inspector`](https://crates.io/crates/gai-inspector) | The CLI |

## Contributing / releasing

See [CHANGELOG.md](CHANGELOG.md) for what's shipped. `scripts/release.sh
<version>` bumps the workspace version, regenerates `Cargo.lock`, commits,
tags, and pushes in one step.

## Scope (MVP)

Linux only. macOS (`scutil`/mDNSResponder) and Windows (DNS Client/LLMNR/
NetBIOS) are architecturally different enough that they're deliberately
out of scope for now rather than bolted on. IPv6 mDNS (AAAA) queries are
not yet implemented.

## License

MIT — see [LICENSE](LICENSE).
