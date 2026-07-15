# Changelog

All notable changes to this project are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
versioning follows [SemVer](https://semver.org/).

## [Unreleased]

## [0.1.3] - 2026-07-15
### Fixed
- `gai-probe`: disabled hickory-resolver's implicit `/etc/hosts` lookup
  (`ResolverOpts::use_hosts_file`), which silently contaminated the `dns`
  NSS source and the reality-check with `/etc/hosts` entries, making a
  Files-resolved name always spuriously "match direct DNS" regardless of
  what DNS actually said.

## [0.1.2] - 2026-07-15
### Added
- `gai-probe`: one-shot mDNS A-record probe (`224.0.0.251:5353`, hand-rolled
  DNS wire format, no dependency) answering the `mdns4_minimal`/`mdns4` NSS
  sources for real instead of a permanent `Skipped`.
### Fixed
- `doctor`: replaced a tuple-match with a silent catch-all arm that could
  report "matches direct DNS" even when the reality check never ran
  (`Resolver::new` failure or query error), with explicit handling for
  every `(resolved, halted_early, reality_result)` combination.
### CI
- `release.yml`: fails the build if the git tag doesn't match
  `Cargo.toml`'s workspace version, instead of silently shipping a binary
  that reports the wrong `--version`.

## [0.1.1] - 2026-07-14
### Added
- `doctor`: prints which nameservers the reality check actually used and
  whether the systemd-resolved stub was detected, to make the D-Bus path
  vs. resolv.conf fallback distinguishable during testing.

## [0.1.0] - 2026-07-14
### Added
- Initial release: `gai explain` / `gai doctor` / `gai why`.
- `gai-core`: pure parsers for `/etc/nsswitch.conf`, `/etc/resolv.conf`,
  `/etc/gai.conf`, `/etc/hosts`, and a `simulate()` emulator that walks the
  `hosts:` NSS chain exactly like glibc, including `[STATUS=action]`
  criteria (e.g. `[NOTFOUND=return]`).
- `gai-probe`: real answers for the `files`/`dns` NSS sources via
  `hickory-resolver`, a `systemd-resolved` D-Bus client for the real
  per-link nameservers hidden behind the `127.0.0.53` stub, an independent
  reality-check DNS query, and best-effort detection of statically linked
  Go binaries that bypass NSS entirely.
- CI (fmt/clippy/test) and a release workflow producing a static
  `x86_64-unknown-linux-musl` binary, plus `install.sh` for one-line
  installs on a VPS.

[Unreleased]: https://github.com/casablanque-code/gai/compare/v0.1.3...HEAD
[0.1.3]: https://github.com/casablanque-code/gai/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/casablanque-code/gai/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/casablanque-code/gai/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/casablanque-code/gai/releases/tag/v0.1.0
