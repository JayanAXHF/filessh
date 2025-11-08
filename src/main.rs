use std::sync::Arc;

use crate::ssh::Session;
use crate::{cli::Cli, files::FileEntry};
use clap::Parser;
use color_eyre::eyre::{self, Result};
use russh_sftp::client::SftpSession;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tracing::info;

mod cli;
mod files;
mod logging;
// mod par_dir_traversal;
mod ssh;
mod tui;

fn main() -> Result<()> {
    logging::init()?;

    info!("Starting...");
    let cli = Cli::parse();

    info!("Connecting to {}:{}", cli.host, cli.port);
    info!("Key path: {:?}", cli.private_key);
    info!("OpenSSH Certificate path: {:?}", cli.openssh_certificate);

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let sftp = rt.block_on(async {
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
        eyre::Ok(sftp)
    })?;
    let sftp = Arc::new(sftp);
    crate::tui::tui(cli.path.display().to_string(), cli, rt, sftp)?;
    Ok(())
}
