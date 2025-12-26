use clap::CommandFactory;
use clap_complete::Shell;
use color_eyre::eyre::{Result, WrapErr, eyre};
use directories::BaseDirs;
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Produce a completion file buffer for the given shell.
pub fn generate_completion(shell: Shell) -> Result<(String, Vec<u8>)> {
    let mut cmd = crate::Cli::command();
    let name = cmd.get_name().to_string();

    let mut buf = Vec::new();
    clap_complete::generate(shell, &mut cmd, name.as_str(), &mut buf);

    Ok((name, buf))
}

/// Install completions to the proper OS-dependent directory.
pub fn install_completions(shell: Shell) -> Result<()> {
    let (bin_name, buf) = generate_completion(shell)?;

    let target_dir = completion_dir(shell)
        .ok_or_else(|| eyre!("No known completion directory for shell: {shell:?}"))?;

    fs::create_dir_all(&target_dir)
        .wrap_err_with(|| format!("failed to create {}", target_dir.display()))?;

    let file_path = completion_file_path(shell, &target_dir, &bin_name);

    fs::write(&file_path, buf)
        .wrap_err_with(|| format!("failed to write completion file to {}", file_path.display()))?;

    println!("Installed {shell:?} completions to {}", file_path.display());

    Ok(())
}

/// Detect appropriate completion directory based on OS conventions.
pub fn completion_dir(shell: Shell) -> Option<PathBuf> {
    let base = BaseDirs::new()?;

    match shell {
        Shell::Bash => Some(base.data_dir().join("bash-completion/completions")),

        Shell::Zsh => {
            // macOS (Homebrew)
            #[cfg(target_os = "macos")]
            {
                let hb1 = PathBuf::from("/opt/homebrew/share/zsh/site-functions");
                let hb2 = PathBuf::from("/usr/local/share/zsh/site-functions");
                if hb1.exists() {
                    return Some(hb1);
                }
                if hb2.exists() {
                    return Some(hb2);
                }
            }
            // Fallback XDG
            Some(base.data_dir().join("zsh/site-functions"))
        }

        Shell::Fish => Some(base.data_dir().join("fish/vendor_completions.d")),

        Shell::PowerShell => Some(base.home_dir().join("Documents/PowerShell/Scripts")),

        Shell::Elvish => Some(base.data_dir().join("elvish/lib")),

        _ => None,
    }
}

/// Determine correct filename for each shell.
pub fn completion_file_path(shell: Shell, dir: &Path, bin_name: &str) -> PathBuf {
    match shell {
        Shell::Zsh => dir.join(format!("_{}", bin_name)),
        Shell::Fish => dir.join(format!("{bin_name}.fish")),
        Shell::PowerShell => dir.join(format!("{bin_name}.ps1")),
        Shell::Elvish => dir.join(format!("{bin_name}.elv")),
        Shell::Bash => dir.join(bin_name),
        _ => unreachable!(),
    }
}

pub fn detect_shell() -> Option<Shell> {
    let shell = std::env::var("SHELL").ok()?;
    let name = Path::new(&shell).file_name()?.to_str()?.to_lowercase();

    match name.as_str() {
        "bash" => Some(Shell::Bash),
        "zsh" => Some(Shell::Zsh),
        "fish" => Some(Shell::Fish),
        "pwsh" | "powershell" => Some(Shell::PowerShell),
        "elvish" => Some(Shell::Elvish),
        _ => None,
    }
}
