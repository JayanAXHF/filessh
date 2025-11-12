# FileSSH

[![Built With Ratatui](https://ratatui.rs/built-with-ratatui/badge.svg)](https://ratatui.rs/)
![crates.io](https://img.shields.io/crates/v/filessh)
![GitHub Tag](https://img.shields.io/github/v/tag/jayanaxhf/filessh)


A TUI-based file explorer for SSH servers.

![Made with VHS](https://vhs.charm.sh/vhs-3VcUb6HOgvuo8mtaeDbRSe.gif)

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

### Arguments

-   `<HOST>`: The hostname or IP address of the SSH server.
-   `<PATH>`: The starting path to explore on the remote server.

### Options

-   `--port <PORT>`: The port number for the SSH connection (default: 22).
-   `--username <USERNAME>`: The username for the SSH connection (default: "root").
-   `-k`, `--private-key <PRIVATE_KEY>`: The path to your SSH private key.
-   `-o`, `--openssh-certificate <OPENSSH_CERTIFICATE>`: The path to your OpenSSH certificate.

### Example

```sh
./target/release/filessh \
    --username myuser \
    --private-key ~/.ssh/id_rsa \
    example.com \
    /home/myuser
```
