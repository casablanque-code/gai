//! gai-core: pure logic for parsing the OS name-resolution configuration
//! and simulating the getaddrinfo() decision path.
//!
//! No network I/O, no process interception. Everything here is testable
//! against fixture files, which is the whole point of keeping it separate
//! from gai-probe.

pub mod config;
pub mod platform;
pub mod sim;
pub mod types;

pub use sim::{simulate, SimulationOutcome};
pub use types::*;
