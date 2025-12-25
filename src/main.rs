use std::sync::Arc;

use crate::cli::{Cli, Commands};
use crate::completions::detect_shell;
use crate::ssh::Session;
use async_lock::Mutex as AsyncMutex;
use clap::Parser;
use color_eyre::eyre::{self, Result, eyre};
use tracing::info;

mod cli;
mod completions;
mod config;
mod errors;
mod files;
mod hosts_parser;
mod logging;
mod par_dir_traversal;
mod patched_line_gauge;
mod ssh;
mod tui;

fn main() -> Result<()> {
    let config = config::Settings::new()?;
    let logging_config = (&config).into();
    logging::init(logging_config)?;

    info!("Starting...");
    let cli = Cli::parse();
    errors::init()?;
    match cli.command.clone() {
        Some(Commands::InstallManPages) => return config::install_manpages(),
        Some(Commands::InstallCompletions { shell }) => {
            use clap_complete::Shell;

            let shell = if shell == "auto" {
                detect_shell().ok_or_else(|| eyre!("Could not auto-detect shell"))?
            } else {
                shell.parse::<Shell>().map_err(|_| eyre!("Unknown shell"))?
            };

            return crate::completions::install_completions(shell);
        }
        Some(Commands::InitConfig) => {
            config::init_config()?;
            return Ok(());
        }
        _ => {}
    }

    let cli = match cli.command {
        Some(Commands::Connect(cli)) => cli,
        None => cli.connect,
        _ => unreachable!(),
    };
    let cli = cli.resolve()?;

    info!("Connecting to {}:{}", cli.host, cli.port);
    info!("Key path: {:?}", cli.private_key);
    info!("OpenSSH Certificate path: {:?}", cli.openssh_certificate);

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let (session, sftp) = rt.block_on(async {
        let cli = cli.clone();
        let mut ssh = Session::connect(
            cli.private_key,
            cli.username.unwrap_or("root".to_string()),
            cli.openssh_certificate,
            (cli.host, cli.port),
        )
        .await?;
        info!("Connected");
        let sftp = ssh.sftp().await?;
        sftp.set_timeout(60000).await;
        eyre::Ok((ssh, sftp))
    })?;
    let sftp = Arc::new(sftp);
    let session = Arc::new(AsyncMutex::new(session));

    crate::tui::tui(
        cli.path.display().to_string(),
        cli,
        rt,
        sftp,
        session,
        config.get_theme(),
    )?;
    eyre::Ok(())
}
