//! gai-probe: supplies real answers for each NSS source and runs the
//! independent "reality check" DNS query used to catch discrepancies.
//!
//! Everything here is I/O. gai-core stays pure so the decision logic can
//! be unit-tested without a network or a filesystem fixture per test.
//! Reminder posted above the monitor: NO EBPF, NO RUNTIME INTERCEPTION.
//! This crate reads configuration and asks resolvers questions — it never
//! attaches to a running process.

pub mod mdns;
pub mod reality;
pub mod resolved;
pub mod resolver;
pub mod runtime;

pub use reality::RealityCheck;
pub use resolved::{query_nameservers, ResolvedNameserver};
pub use resolver::SystemSourceResolver;
pub use runtime::{detect_resolver_runtime, ResolverRuntime};
