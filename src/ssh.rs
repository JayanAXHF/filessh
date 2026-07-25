use std::borrow::Cow;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use color_eyre::Result;
use color_eyre::eyre::{Context, bail};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use russh::keys::*;
use russh::*;
use russh_sftp::client::SftpSession;
use tokio::net::ToSocketAddrs;
use tracing::debug;

/// How many times to ask for a passphrase before giving up, as in `ssh`.
const PASSPHRASE_ATTEMPTS: usize = 3;

/// Loads a private key, asking for the passphrase if the key turns out to be
/// encrypted. `russh` reports that as [`keys::Error::KeyIsEncrypted`] rather
/// than prompting itself.
fn load_secret_key_interactive(key_path: &Path) -> Result<PrivateKey> {
    match load_secret_key(key_path, None) {
        Ok(key) => return Ok(key),
        Err(keys::Error::KeyIsEncrypted) => {}
        Err(error) => {
            return Err(error).wrap_err_with(|| {
                format!("could not load the private key {}", key_path.display())
            });
        }
    }

    for attempt in 1..=PASSPHRASE_ATTEMPTS {
        let passphrase = prompt_passphrase(key_path)?;
        match load_secret_key(key_path, Some(&passphrase)) {
            Ok(key) => return Ok(key),
            Err(error) if attempt < PASSPHRASE_ATTEMPTS => {
                // Almost always a mistyped passphrase, but name the real cause
                // so a corrupt or unsupported key cannot masquerade as one.
                eprintln!(
                    "Bad passphrase ({error}), try again for key '{}'",
                    key_path.display()
                );
            }
            Err(error) => {
                return Err(error).wrap_err_with(|| {
                    format!("could not decrypt the private key {}", key_path.display())
                });
            }
        }
    }
    unreachable!("the last attempt returns")
}

/// Reads a passphrase from the terminal without echoing it.
fn prompt_passphrase(key_path: &Path) -> Result<String> {
    /// Leaves raw mode however the read ends, including on `?`.
    struct RawMode;
    impl Drop for RawMode {
        fn drop(&mut self) {
            let _ = disable_raw_mode();
        }
    }

    eprint!("Enter passphrase for key '{}': ", key_path.display());
    std::io::stderr().flush()?;

    enable_raw_mode().wrap_err("a terminal is needed to read the key passphrase")?;
    let _raw_mode = RawMode;

    let mut passphrase = String::new();
    loop {
        let Event::Key(key) = event::read()? else {
            continue;
        };
        // Windows also reports releases and repeats.
        if key.kind != KeyEventKind::Press {
            continue;
        }
        match (key.code, key.modifiers) {
            (KeyCode::Enter, _) => break,
            (KeyCode::Backspace, _) => {
                passphrase.pop();
            }
            (KeyCode::Esc, _) | (KeyCode::Char('c' | 'd'), KeyModifiers::CONTROL) => {
                // Still in raw mode, so end the prompt line by hand.
                eprint!("\r\n");
                bail!("passphrase entry cancelled");
            }
            (KeyCode::Char(c), modifiers) if !modifiers.contains(KeyModifiers::CONTROL) => {
                passphrase.push(c);
            }
            _ => {}
        }
    }
    eprint!("\r\n");
    std::io::stderr().flush()?;

    Ok(passphrase)
}

struct Client {}

// More SSH event handlers
// can be defined in this trait
// In this example, we're only using Channel, so these aren't needed.
impl client::Handler for Client {
    type Error = color_eyre::Report;

    async fn check_server_key(
        &mut self,
        server_public_key: &ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        debug!("check_server_key: {server_public_key:?}");
        Ok(true)
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        _session: &mut client::Session,
    ) -> Result<(), Self::Error> {
        debug!("data on channel {:?}: {}", channel, data.len());
        Ok(())
    }
}

/// This struct is a convenience wrapper
/// around a russh client
pub struct Session {
    session: client::Handle<Client>,
}

impl Session {
    pub async fn connect<P: AsRef<Path>, A: ToSocketAddrs>(
        key_path: P,
        user: impl Into<String>,
        openssh_cert_path: Option<P>,
        addrs: A,
    ) -> Result<Self> {
        let key_pair = load_secret_key_interactive(key_path.as_ref())?;

        // load ssh certificate
        let openssh_cert = openssh_cert_path
            .map(load_openssh_certificate)
            .transpose()
            .wrap_err("could not load the OpenSSH certificate")?;

        let config = client::Config {
            inactivity_timeout: Some(Duration::from_secs(500000)),
            preferred: Preferred {
                kex: Cow::Owned(vec![
                    russh::kex::CURVE25519_PRE_RFC_8731,
                    russh::kex::EXTENSION_SUPPORT_AS_CLIENT,
                ]),
                ..Default::default()
            },
            ..<_>::default()
        };

        let config = Arc::new(config);
        let sh = Client {};

        let mut session = client::connect(config, addrs, sh).await?;
        // use publickey authentication, with or without certificate
        if let Some(openssh_cert) = openssh_cert {
            let auth_res = session
                .authenticate_openssh_cert(user, Arc::new(key_pair), openssh_cert)
                .await?;

            if !auth_res.success() {
                bail!("Authentication (with publickey+cert) failed");
            }
        } else {
            let auth_res = session
                .authenticate_publickey(
                    user,
                    PrivateKeyWithHashAlg::new(
                        Arc::new(key_pair),
                        session.best_supported_rsa_hash().await?.flatten(),
                    ),
                )
                .await?;

            if !auth_res.success() {
                bail!("Authentication (with publickey) failed");
            }
        }

        Ok(Self { session })
    }

    pub async fn sftp(&mut self) -> Result<SftpSession> {
        let channel = self.session.channel_open_session().await?;
        channel.request_subsystem(true, "sftp").await?;
        let sftp = SftpSession::new(channel.into_stream()).await?;
        Ok(sftp)
    }

    pub async fn close(&mut self) -> Result<()> {
        self.session
            .disconnect(Disconnect::ByApplication, "", "English")
            .await?;
        Ok(())
    }
}
