mod definition;
use color_eyre::eyre::{Context, Result, eyre};
pub use definition::*;
use std::path::Path;

use crate::ssh_config::{self, Hosts, reader::SSHConfigReader};

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
        if let Some(host) = &self.from_config {
            let mut config_reader = SSHConfigReader::new();

            config_reader.read()?;
            let config = config_reader.finalize();
            let config: Hosts = ssh_config::from_str(&config)?;
            let host_config = config.0.iter().find(|h| h.host_name == host);
        }
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
