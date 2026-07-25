# FileSSH

[![Built With Ratatui](https://ratatui.rs/built-with-ratatui/badge.svg)](https://ratatui.rs/)
![crates.io](https://img.shields.io/crates/v/filessh)
![GitHub Tag](https://img.shields.io/github/v/tag/jayanaxhf/filessh)

A TUI-based file explorer for SSH servers, which allows you to browse and manage files on a remote server, edit them in-place, and recursively download directories with parallel directory traversal. It also has the ability to quickly spawn SSH sessions to paths on the remote server.

Dual-licensed under MIT or the [UNLICENSE](https://unlicense.org/).

![Made with VHS](https://vhs.charm.sh/vhs-3OLXZvjKpqe5qR7hxsftQF.gif)

## Installation

### homebrew (macOS)

```bash
brew install jayanaxhf/taps/filessh
```

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

### Keybindings

**To quit, press <kbd>Ctrl</kbd>+<kbd>q</kbd>.**

The interface has three panes: the file list, the metadata and file content
pane, and the input box that prompts appear in.

| Key | Action |
| --- | --- |
| <kbd>Ctrl</kbd>+<kbd>q</kbd> | Quit |
| <kbd>Tab</kbd> / <kbd>Shift</kbd>+<kbd>Tab</kbd> | Move focus between the panes |
| <kbd>1</kbd> / <kbd>2</kbd> / <kbd>3</kbd> | Focus the file list, the content pane, or the input box |
| <kbd>Esc</kbd> | Return to the file list, clearing any prompt or filter |

In the file list:

| Key | Action |
| --- | --- |
| <kbd>j</kbd> / <kbd>k</kbd>, <kbd>↓</kbd> / <kbd>↑</kbd> | Move the selection |
| <kbd>l</kbd> / <kbd>→</kbd> | Enter the selected directory |
| <kbd>h</kbd> / <kbd>←</kbd> | Go to the parent directory |
| <kbd>Enter</kbd> | Show the selected file's contents in the content pane |
| <kbd>e</kbd> | Edit the file in `$EDITOR`, after <kbd>Enter</kbd> has loaded it |
| <kbd>d</kbd> | Download the selection, prompting for a local path |
| <kbd>f</kbd> | Filter the list by name, as you type |
| <kbd>m</kbd> | Rename or move the selection |
| <kbd>x</kbd> | Delete the selection, confirming with <kbd>y</kbd> |
| <kbd>n</kbd> <kbd>f</kbd> | Create a file |
| <kbd>n</kbd> <kbd>d</kbd> | Create a directory |
| <kbd>.</kbd> | Hide dotfiles, which are shown by default |
| <kbd>Ctrl</kbd>+<kbd>o</kbd> | Open an SSH session in the current directory |

At a prompt, <kbd>Enter</kbd> accepts and <kbd>Esc</kbd> cancels.
