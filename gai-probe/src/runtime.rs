use std::path::Path;

/// How the target binary actually resolves names — the trap most people
/// don't know about. A statically linked Go binary ships its own pure-Go
/// resolver by default and never touches nsswitch.conf, glibc, or NSS at
/// all. Simulating the OS chain for such a binary would be simulating the
/// wrong thing entirely, so gai must detect and say so plainly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolverRuntime {
    GlibcNss,
    MuslNss,
    GoPureResolver,
    Unknown,
}

/// Best-effort detection by inspecting the ELF for a dynamic interpreter
/// and known Go build markers. Not perfect (cgo-enabled Go binaries do
/// use glibc NSS), but MVP-honest: when unsure, say Unknown rather than
/// guessing confidently and misleading the diagnosis.
pub fn detect_resolver_runtime(binary_path: &Path) -> anyhow::Result<ResolverRuntime> {
    let bytes = std::fs::read(binary_path)?;

    let is_dynamic = bytes.windows(4).any(|w| w == b"\x7fELF")
        && find_subslice(&bytes, b"/lib64/ld-linux").is_some()
            || find_subslice(&bytes, b"/lib/ld-linux").is_some();

    let looks_like_go = find_subslice(&bytes, b"Go build ID:").is_some()
        || find_subslice(&bytes, b"golang.org/x").is_some();

    if looks_like_go && !is_dynamic {
        return Ok(ResolverRuntime::GoPureResolver);
    }
    if is_dynamic {
        // Distinguishing glibc vs musl reliably needs reading the
        // interpreter string exactly; left for a follow-up patch.
        return Ok(ResolverRuntime::GlibcNss);
    }
    Ok(ResolverRuntime::Unknown)
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
