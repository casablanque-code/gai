use clap::{Parser, Subcommand};

mod doctor;
mod explain;

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
    Doctor { name: String },
    /// Alias for `doctor`, phrased as a question — same output.
    Why { name: String },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Explain { name } => explain::run(&name),
        Command::Doctor { name } | Command::Why { name } => doctor::run(&name),
    }
}
