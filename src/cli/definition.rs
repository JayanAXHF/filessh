//! The CLI Module is structured this way
//! to allow for the buils.rs script to
//! generate the man pages and completions
//! for the CLI at build time.
use clap::Parser;
use clap::command;
use clap::crate_authors;
use std::path::PathBuf;

use std::sync::LazyLock;

pub static SHORT_VERSION: LazyLock<&'static str> = LazyLock::new(|| {
    let v = format!(
        "{}-{} ({})\nWritten by {}",
        env!("CARGO_PKG_VERSION"),
        option_env!("VERGEN_GIT_DESCRIBE").unwrap_or("unknown"),
        option_env!("VERGEN_BUILD_DATE").unwrap_or("unknown"),
        crate_authors!(),
    );

    // Leak into a &'static str
    Box::leak(v.into_boxed_str())
});

/// Filessh: A small SSH-based remote file browser
#[derive(Parser, Debug, Default)]
#[command(
    version = *SHORT_VERSION,
    about,
    propagate_version = true,
    disable_help_subcommand = true,
    args_conflicts_with_subcommands = true
)]
pub struct Cli {
    /// Optional subcommand
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Default command arguments (flattened)
    #[command(flatten)]
    pub connect: ConnectArgs,
}

/// All subcommands
#[derive(clap::Subcommand, Debug, Clone)]
pub enum Commands {
    /// Connect explicitly (same as default command)
    Connect(ConnectArgs),

    /// Install man pages into the system
    InstallManPages,

    /// Generate a default config file to the default location
    InitConfig,

    /// Generate shell completion scripts
    InstallCompletions {
        /// Shell name (bash, zsh, fish)
        #[clap(default_value = "bash")]
        shell: String,
    },
}

/// Arguments for the default “connect” command
#[derive(clap::Args, Debug, Clone, Default)]
pub struct ConnectArgs {
    /// The remote host to connect to (e.g., 'example.com' or '192.168.1.100').
    #[clap(index = 1)]
    pub host: Option<String>,

    /// The port number to use for the SSH connection.
    #[clap(long, short, default_value_t = 22)]
    pub port: u16,

    /// The username for logging into the remote host.
    #[clap(long, short)]
    pub username: Option<String>,

    /// Path to the private key file for public key authentication.
    #[clap(long, short = 'k')]
    pub private_key: Option<PathBuf>,

    /// Optional path to an OpenSSH certificate.
    #[clap(long, short = 'o')]
    pub openssh_certificate: Option<PathBuf>,

    /// Initial directory path to open on the remote host.
    #[clap(index = 2)]
    pub path: Option<PathBuf>,

    #[clap(short, long)]
    pub from_config: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ResolvedConnectArgs {
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub private_key: PathBuf,
    pub openssh_certificate: Option<PathBuf>,
    pub path: PathBuf,
}
