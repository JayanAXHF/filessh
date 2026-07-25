mod definition;
use color_eyre::eyre::{Context, Result, eyre};
pub use definition::*;
use std::path::{Path, PathBuf};
use tracing::debug;

use crate::ssh_config::{self, Host, Hosts, reader::SSHConfigReader};

const DEFAULT_SSH_PORT: u16 = 22;

/// The account `ssh` would log in as when nothing names one.
fn local_username() -> Option<String> {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .ok()
        .filter(|name| !name.is_empty())
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
        if self.from_config {
            let host = self
                .host
                .as_ref()
                .ok_or_else(|| eyre!("missing required argument: <host>"))?;
            let mut config_reader = SSHConfigReader::new();

            config_reader.read()?;
            let config = config_reader.finalize();
            let config: Hosts = ssh_config::from_str(&config)?;
            let Some(host_config) = config.settings_for(host) else {
                return Err(eyre!("Host not found in config file"));
            };
            let host_config = &host_config;
            let path = self
                .path
                .as_ref()
                .ok_or_else(|| eyre!("missing required argument: <path>"))
                .wrap_err("You must provide a path. Example: filessh example.com /var/www")?
                .clone();
            let Host {
                host_name,
                user,
                port,
                identity_file,
                name: _,
            } = host_config;

            // Each of these keywords is optional in the config, so fall back
            // the way ssh does: an absent HostName means the alias is itself
            // the host name, and command line flags win over the config.
            let private_key = match (self.private_key.clone(), identity_file.as_deref()) {
                (Some(path), _) => path,
                (None, Some(file)) => {
                    let expanded = shellexpand::full(file)?;
                    PathBuf::from(expanded.as_ref())
                        .canonicalize()
                        .wrap_err_with(|| format!("IdentityFile {file} of Host {host}"))?
                }
                (None, None) => {
                    return Err(eyre!(
                        "Host {host} has no IdentityFile in your SSH config; pass --private-key"
                    ));
                }
            };
            debug!("pvt_key_path: {:?}", private_key);

            return Ok(ResolvedConnectArgs {
                host: host_name.clone().unwrap_or_else(|| host.clone()),
                port: port.unwrap_or(DEFAULT_SSH_PORT),
                // Without a `User` anywhere, ssh logs in as whoever is running
                // it. Leaving this unset would reach the `root` fallback in
                // `main` instead, which is nobody's account by default.
                username: self
                    .username
                    .clone()
                    .or_else(|| user.clone())
                    .or_else(local_username),
                private_key,
                openssh_certificate: self.openssh_certificate.clone(),
                path,
            });
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
