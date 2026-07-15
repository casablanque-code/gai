//! gai-core: pure logic for parsing the OS name-resolution configuration
//! and simulating the getaddrinfo() decision path.
//!
//! No network I/O, no process interception. Everything here is testable
//! against fixture files, which is the whole point of keeping it separate
//! from gai-probe, which supplies real answers over the network/D-Bus.
//!
//! # Example
//!
//! ```
//! use gai_core::sim::{simulate, SourceResolver};
//! use gai_core::types::{NssEntry, NssSource, NsswitchConfig, StepResult};
//!
//! // A resolver that always finds an answer via DNS — in real use this
//! // would be gai-probe's SystemSourceResolver, doing actual I/O.
//! struct AlwaysDns;
//! impl SourceResolver for AlwaysDns {
//!     fn resolve(&mut self, source: &NssSource, _name: &str) -> StepResult {
//!         match source {
//!             NssSource::Dns => StepResult::Found(vec!["93.184.216.34".parse().unwrap()]),
//!             _ => StepResult::NotFound,
//!         }
//!     }
//! }
//!
//! let config = NsswitchConfig {
//!     hosts: vec![
//!         NssEntry { source: NssSource::Files, criteria: vec![] },
//!         NssEntry { source: NssSource::Dns, criteria: vec![] },
//!     ],
//! };
//!
//! let outcome = simulate(&config, "example.com", &mut AlwaysDns);
//! assert!(outcome.resolved());
//! assert_eq!(outcome.steps.len(), 2, "files was tried and fell through to dns");
//! ```

pub mod config;
pub mod platform;
pub mod sim;
pub mod types;

pub use sim::{simulate, SimulationOutcome};
pub use types::*;
