use std::io::Read;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::sync::LazyLock;
#[cfg(target_os = "windows")]
use std::sync::LazyLock;

use color_eyre::eyre::Result;

pub struct SSHConfigReader {
    buf: String,
}

#[cfg(target_os = "windows")]
const SSH_CONFIG_PATHS: LazyLock<[String; 1]> = LazyLock::new(|| {
    let mut path = std::env::var("userprofile").unwrap();
    path.push_str(r#"\.ssh\config"#);
    [path]
});

#[cfg(any(target_os = "linux", target_os = "macos"))]
const SSH_CONFIG_PATH: LazyLock<[String; 2]> = LazyLock::new(|| {
    let mut path = std::env::var("HOME").unwrap();
    path.push_str(r#"/.ssh/config"#);
    [path, "/etc/ssh/ssh_config".to_string()]
});

impl SSHConfigReader {
    pub fn new() -> Self {
        Self { buf: String::new() }
    }

    pub fn read(&mut self) -> Result<()> {
        let paths = SSH_CONFIG_PATH.clone();
        for path in paths {
            if std::path::Path::new(&path).exists() {
                let f = std::fs::File::open(path)?;
                std::io::BufReader::new(f).read_to_string(&mut self.buf)?;
            }
        }
        Ok(())
    }
    pub fn finalize(self) -> String {
        self.buf
    }
}
