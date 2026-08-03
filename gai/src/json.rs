use serde::{Serialize, Serializer};
use std::net::IpAddr;

/// JSON-serializable result for `explain` subcommand.
#[derive(Serialize)]
pub struct ExplainOutput {
    pub name: String,
    pub resolved: bool,
    pub addresses: Vec<String>,
    pub steps: Vec<ResolutionStepJson>,
}

/// JSON-serializable result for `doctor` subcommand.
#[derive(Serialize)]
pub struct DoctorOutput {
    pub name: String,
    pub resolved: bool,
    pub addresses: Vec<String>,
    pub steps: Vec<ResolutionStepJson>,
    pub reality_check: RealityCheckJson,
    pub diagnosis: DiagnosisJson,
}

/// One step in the resolution path.
#[derive(Serialize)]
pub struct ResolutionStepJson {
    pub source: String,
    pub status: String,
    pub addresses: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub halted_chain: Option<HaltedChainJson>,
}

/// Information about the reality check (direct DNS query).
#[derive(Serialize)]
pub struct RealityCheckJson {
    pub resolved: bool,
    pub addresses: Vec<String>,
    pub nameservers: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Diagnosis verdict and message.
#[derive(Serialize)]
pub struct DiagnosisJson {
    pub severity: String, // "ok", "note", or "issue"
    pub message: String,
}

/// Information about what halted the resolution chain.
#[derive(Serialize)]
pub struct HaltedChainJson {
    pub status: String,
    pub action: String,
}

/// Helper to serialize IpAddr as string.
pub fn serialize_ip_addr<S>(addr: &IpAddr, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&addr.to_string())
}

/// Convert a vec of IpAddr to vec of strings.
pub fn ips_to_strings(ips: &[IpAddr]) -> Vec<String> {
    ips.iter().map(|ip| ip.to_string()).collect()
}
