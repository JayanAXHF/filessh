use clap::Parser;
use clap::command;
use color_eyre::eyre::Context;
use color_eyre::eyre::Result;
use color_eyre::eyre::eyre;
use std::path::{Path, PathBuf};

/// Filessh: A small SSH-based remote file browser
#[derive(Parser, Debug, Default)]
#[command(
    version,
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

impl ResolvedConnectArgs {
    /// Build a base SSH command (no remote path yet)
    pub fn build_ssh_command(&self) -> std::process::Command {
        type Command = std::process::Command;
        let mut cmd = Command::new("ssh");

        if let Some(username) = &self.username {
            cmd.arg("-l").arg(username);
        }

        cmd.arg("-p").arg(self.port.to_string());
        cmd.arg("-i").arg(self.private_key.display().to_string());

        // Use user@host or fallback to "root@host"
        let user = self.username.as_deref().unwrap_or("root");
        cmd.arg(format!("{user}@{}", self.host));

        cmd
    }

    /// Build SSH command that opens into the given remote path
    pub fn build_ssh_with_path<P>(&self, path: P) -> std::process::Command
    where
        P: AsRef<Path>,
    {
        let mut cmd = self.build_ssh_command();

        // Build remote command: cd <path>; bash --login
        let remote_cmd = format!("cd {}; bash --login", path.as_ref().display());
        cmd.arg("-t").arg(remote_cmd);

        cmd
    }
}

impl ConnectArgs {
    pub fn resolve(&self) -> Result<ResolvedConnectArgs> {
        let host = self
            .host
            .as_ref()
            .ok_or_else(|| eyre!("missing required argument: <host>"))
            .wrap_err("You must provide a host. Example: filessh example.com .")?
            .clone();

        let path = self
            .path
            .as_ref()
            .ok_or_else(|| eyre!("missing required argument: <path>"))
            .wrap_err("You must provide a path. Example: filessh example.com /var/www")?
            .clone();

        let private_key = self
            .private_key
            .as_ref()
            .ok_or_else(|| eyre!("missing --private-key <FILE>"))
            .wrap_err("The private key flag (-k, --private-key) is required.")?
            .clone();

        Ok(ResolvedConnectArgs {
            host,
            port: self.port,
            username: self.username.clone(),
            private_key,
            openssh_certificate: self.openssh_certificate.clone(),
            path,
        })
    }
}
