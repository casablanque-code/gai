# Changelog

All notable changes to this project are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
versioning follows [SemVer](https://semver.org/).

## [0.5.1] - 2026-08-12
### Fixed
- `nsswitch.conf` parser: support negated `[!STATUS=action]` criteria.
  Previously errored with `unknown status '!UNAVAIL'` on authselect's
  default `hosts:` line — the standard config on every Fedora/RHEL/CentOS
  install, so this hit any user on those distros running `gai` at all.
- `mdns4_minimal`/`mdns4`/`mdns6_minimal`/`mdns6`: gate the mDNS probe to
  names ending in `.local`, matching real nss-mdns. Previously an ordinary
  internet domain (e.g. `google.com`) triggered a real multicast query,
  got no answer, and was misreported as `NOT FOUND` — tripping the common
  `[NOTFOUND=return]` clause and producing a false `ISSUE` diagnosis for
  a domain that was never actually broken.
- README: documented the `.local` mDNS gating above, and corrected two
  stale "IPv6/AAAA not yet implemented" claims — `mdns6`/`mdns6_minimal`
  support already existed in code.

## [0.5.0] - 2026-08-11
### Added
- `doctor`/`why`: new `--binary <path>` flag. Detects statically linked Go
  binaries (which ship their own pure-Go resolver and bypass NSS entirely)
  and warns that the simulated OS chain doesn't apply to them. Previously
  this detection existed in `gai-probe` but was never wired into the CLI.

### Fixed
- README: documented that `install.sh` always installs to `/usr/local/bin`
  with mode `0755` and doesn't support a custom output path.

## [0.4.0] - 2026-07-31
### Added
- `explain`/`doctor`: colored, boxed CLI output. Each resolution step now
  gets a FOUND/NOT FOUND/SKIPPED tag (green/dim/red), and the final
  verdict is shown in a bordered panel color-accented by severity — green
  for no issue, yellow for informational only (anycast variance, couldn't
  cross-check), red for something worth investigating. `doctor`'s panel
  title doubles as a one-word verdict (OK/NOTE/ISSUE). Auto-disables for
  non-terminal output (pipes, redirects) or when `NO_COLOR` is set, so
  scripting against `gai`'s output is unaffected.

### Fixed
- Output panel borders (top/content/bottom) didn't agree on width, so
  boxes rendered visibly lopsided — the bottom border came out 2 columns
  wider than the top.

## [0.3.1] - 2026-07-31
### Fixed
- `doctor`/`explain`: a DNS query error (timeout, SERVFAIL, ...) was
  indistinguishable from an authoritative NXDOMAIN — both silently became
  `NotFound`. Now split via NetError::is_no_records_found(): a real
  negative answer stays `NotFound`, everything else surfaces as `Skipped`
  with the underlying reason.
- `explain`/`doctor`: nameservers from resolved's Manager.DNS were pooled
  flat across all links, so a link-scoped-only resolver (e.g. a VPN's
  split-DNS server) could be queried for public names outside its zone
  and SERVFAIL. Global-scope (ifindex 0) servers are now preferred.

### Known limitations
- Per-link/split-DNS domain routing (a name being routed to the specific
  link's resolver based on systemd-resolved's search domains) is not
  implemented — `gai` queries global-scope nameservers only. A name that
  only resolves via a VPN/link-scoped resolver (e.g. Tailscale's
  `*.ts.net`) will correctly report NOT FOUND rather than silently lying,
  but won't actually resolve it either. Tracked as a follow-up.

## [0.3.0] - 2026-07-31
### Fixed
- `doctor`/`explain`: a transient DNS query error (timeout, refused, ...) was
  being silently treated as an authoritative NXDOMAIN instead of surfacing as
  "the check couldn't run" — could cause `doctor` to falsely report a resolved
  name as failing, or falsely agree/disagree with DNS reality.
- `doctor`: resolved addresses were compared as ordered lists, causing a false
  "disagreement" diagnosis whenever DNS and the OS chain returned the same
  address set in a different order (e.g. `localhost`).
- `doctor`: diagnosis no longer blames files/mdns for a mismatch when the
  simulated chain's own answer came from DNS itself (more likely anycast/
  GeoDNS variance between two separate queries).
- `scripts/release.sh`: internal path-dependency pins between the three
  crates were only bumped on an exact OLD_VERSION match, letting them
  silently drift stale across a release; the bump is now unconditional
  with a hard check that aborts before tagging if any pin is missed.

## [0.2.1] - 2026-07-20
### Fix and add readme publish on crates.io

## [0.2.0] - 2026-07-20
### Added
- IPv6 mDNS (AAAA) probing — `mdns6`/`mdns6_minimal` NSS sources are now
  actually queried instead of being skipped as unimplemented.
- `gai-probe`: unit tests for mDNS packet building/parsing (`build_query`,
  `parse_records`, name compression, pointer-loop guard), covering both
  A and AAAA records and cross-type mismatch handling.
- `gai`: CLI-level integration tests (`assert_cmd`) for argument parsing,
  subcommand dispatch, and `explain`/`doctor`/`why` behavior.
- `.github/dependabot.yml` (cargo + github-actions, weekly) and
  `deny.toml` + a `dependency-audit` CI job (advisories/bans/licenses/
  sources via cargo-deny), with a weekly cron so newly-published RUSTSEC
  advisories are caught even without a push.
- `scripts/check_tag_version.sh` + `scripts/check_tag_version_test.sh` —
  the release tag/version guard is now a standalone, unit-tested script
  instead of untested inline workflow logic.
### Changed
- `hickory-resolver` 0.24 → 0.26.1 (fixes RUSTSEC-2026-0119, an O(n²)
  CPU-exhaustion DoS in `hickory-proto`'s message encoding). This is a
  breaking upstream API change — 0.26 dropped the blocking `Resolver` in
  favor of an async-only one; `resolver.rs` and `reality.rs` now bridge
  it with a short-lived current-thread tokio runtime per query, keeping
  `SourceResolver::resolve`'s synchronous signature intact.

## [0.1.5] - 2026-07-15
### Docs
- `gai-core`: full field/variant-level rustdoc for every public type in
  `types.rs` (previously undocumented individually), plus a compiling
  usage example in the crate-level docs. `gai-probe` and `gai-inspector`
  are re-released at the same version (shared workspace version) with no
  functional changes.

## [0.1.4] - 2026-07-15
### Changed
- CLI package renamed from `gai-cli` to `gai-inspector` — `gai-cli` was
  also already taken on crates.io. The binary itself is still named
  `gai`; `cargo install gai-inspector` installs it.
### Added
- `gai-core`, `gai-probe`, and the CLI now carry crates.io publish
  metadata (description, keywords, categories) and a `publish.yml`
  workflow, gated behind the same tag/version check as `release.yml`.
- `scripts/release.sh`: one command to bump the version, regenerate
  `Cargo.lock`, commit, tag, and push.
- `CHANGELOG.md` (this file).
- `gai-core`: test coverage for `hosts.rs` and `gai_conf.rs` (previously
  untested), plus `sim.rs` edge cases (`NOTFOUND` without a criterion,
  explicit `[SUCCESS=continue]`, full chain exhausted).
- `gai-core/tests/scenarios.rs`: end-to-end tests parsing real
  nsswitch.conf text through `simulate()`, including the
  `mdns4_minimal [NOTFOUND=return]` trap from the original project spec.
### CI
- `ci.yml` now only runs on `*.rs` / `Cargo.toml` / `Cargo.lock` changes,
  so doc-only edits don't trigger the full pipeline.

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

[Unreleased]: https://github.com/casablanque-code/gai/compare/v0.1.5...HEAD
[0.1.5]: https://github.com/casablanque-code/gai/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/casablanque-code/gai/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/casablanque-code/gai/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/casablanque-code/gai/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/casablanque-code/gai/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/casablanque-code/gai/releases/tag/v0.1.0
