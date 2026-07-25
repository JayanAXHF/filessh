//! The CLI Module is structured this way
//! to allow for the buils.rs script to
//! generate the man pages and completions
//! for the CLI at build time.
use clap::crate_authors;
use std::path::PathBuf;
use clap::Parser;

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

    /// Resolve the connection from your SSH config file, treating <HOST> as a
    /// Host alias defined there instead of as a host name.
    #[clap(short, long)]
    pub from_config: bool,
}

#[cfg(test)]
mod tests {
    use super::Cli;
    use clap::CommandFactory;

    /// Returns the body of a `.SH <heading>` section, up to the next `.SH`.
    /// Looking inside the relevant section keeps prose elsewhere in the page
    /// from passing as documentation: "connect to the remote host" in
    /// DESCRIPTION does not document the `connect` subcommand.
    fn section<'a>(man: &'a str, heading: &str) -> &'a str {
        let body = man
            .split_once(&format!(".SH {heading}\n"))
            .unwrap_or_else(|| panic!("man/filessh.1 has no {heading} section"))
            .1;
        body.split_once("\n.SH ").map_or(body, |(body, _)| body)
    }

    /// `filessh install-man-pages` ships the `man/filessh.1` that is checked into
    /// the repository, and the Homebrew formula installs that same file, so the
    /// roff is maintained by hand and can fall behind the CLI. Catch that here.
    #[test]
    fn man_page_documents_every_option_and_command() {
        // Roff escapes `-` as `\-`; drop the escapes so names match literally.
        let man = include_str!("../../man/filessh.1").replace('\\', "");
        // `build` materialises the generated `--help`/`--version` arguments.
        let mut cli = Cli::command();
        cli.build();

        let commands = section(&man, "COMMANDS");
        let options = section(&man, "OPTIONS");

        let mut undocumented = cli
            .get_subcommands()
            .map(|sub| sub.get_name().to_owned())
            .filter(|name| !commands.contains(name))
            .chain(
                cli.get_arguments()
                    .chain(cli.get_subcommands().flat_map(|sub| sub.get_arguments()))
                    .filter_map(|arg| arg.get_long())
                    .map(|long| format!("--{long}"))
                    .filter(|flag| !options.contains(flag)),
            )
            .collect::<Vec<_>>();
        undocumented.sort();
        undocumented.dedup();

        assert!(
            undocumented.is_empty(),
            "man/filessh.1 is out of date with the CLI; undocumented: {undocumented:?}"
        );
    }
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
