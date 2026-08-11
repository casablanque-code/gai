use clap::{Parser, Subcommand};

mod doctor;
mod explain;
mod style;

#[derive(Parser)]
#[command(
    name = "gai",
    version,
    about = "getaddrinfo inspector — explains how a name turns into an IP"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Show the resolution path the OS would walk for NAME, no verdict.
    Explain { name: String },
    /// Show the resolution path plus a diagnosis when it disagrees with reality.
    Doctor {
        name: String,
        /// Path to the binary that will actually be doing the resolving.
        /// Flags it if it's a statically linked Go binary, which bypasses
        /// NSS (and everything this tool simulates) entirely.
        #[arg(long)]
        binary: Option<std::path::PathBuf>,
    },
    /// Alias for `doctor`, phrased as a question — same output.
    Why {
        name: String,
        #[arg(long)]
        binary: Option<std::path::PathBuf>,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Explain { name } => explain::run(&name),
        Command::Doctor { name, binary } | Command::Why { name, binary } => {
            doctor::run(&name, binary.as_deref())
        }
    }
}
