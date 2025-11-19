# FileSSH

[![Built With Ratatui](https://ratatui.rs/built-with-ratatui/badge.svg)](https://ratatui.rs/)
![crates.io](https://img.shields.io/crates/v/filessh)
![GitHub Tag](https://img.shields.io/github/v/tag/jayanaxhf/filessh)


A TUI-based file explorer for SSH servers, which allows you to browse and manage files on a remote server, edit them in-place, and recursively download directories with parallel directory traversal. It also has the ability to quickly spawn SSH sessions to paths on the remote server.

Dual-licensed under MIT or the [UNLICENSE](https://unlicense.org/).

![Made with VHS](https://vhs.charm.sh/vhs-3OLXZvjKpqe5qR7hxsftQF.gif)

## Installation
### Cargo
```sh
cargo install --locked filessh
```
### Build from source

1.  Ensure you have Rust and Cargo installed. You can find installation instructions at [rust-lang.org](https://www.rust-lang.org/tools/install).
2.  Clone the repository:
    ```sh
    git clone https://github.com/your-username/filessh.git
    cd filessh
    ```
3.  Build the project:
    ```sh
    cargo build --release
    ```
    The executable will be located at `target/release/filessh`.

## Todo

- [ ] Add support for rsync and scp
- [ ] Iron out bugs

## Usage

```sh
filessh [OPTIONS] <HOST> <PATH>
```
### Features
1. Modify, delete and browse files on a remote server
2. Recursively download directories with parallel directory traversal
3. Quickly open SSH sessions to directories.

### Usage

```
filessh [OPTIONS] [HOST] [PATH]
filessh <COMMAND>

Commands:
  connect              Connect explicitly (same as default command)
  install-man-pages    Install man pages into the system
  install-completions  Generate shell completion scripts

Arguments:
  [HOST]  The remote host to connect to (e.g., 'example.com' or '192.168.1.100')
  [PATH]  Initial directory path to open on the remote host

Options:
  -p, --port <PORT>
          The port number to use for the SSH connection [default: 22]
  -u, --username <USERNAME>
          The username for logging into the remote host
  -k, --private-key <PRIVATE_KEY>
          Path to the private key file for public key authentication
  -o, --openssh-certificate <OPENSSH_CERTIFICATE>
          Optional path to an OpenSSH certificate
  -h, --help
          Print help
  -V, --version
          Print version
```

### Example

```sh
./target/release/filessh \
    --username myuser \
    --private-key ~/.ssh/id_rsa \
    example.com \
    /home/myuser
```
