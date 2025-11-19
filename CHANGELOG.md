# Changelog
## [0.2.0] - 2025-11-19

### Features

- Added clap-completion support
- *(cli)* [**breaking**] Changed the CLI structure to be modular and enable subcommands like install-man-pages
- *(docs)* Added man pages for the project and the config system
- Added a config system to the project, allowing users to configure themes

## [0.1.10] - 2025-11-17

### Features

- *(TUI)* Added a display for the currect connection

### Bug Fixes

- *(TUI)* Fixed inconsistent border colours, and removed unnecessary whitespace

### Other

- *(docs)* Merge pull request #18 from ZennoZenith/patch-1

### Documentation

- Update the README to include license information and added the relevant license files.
- *(README)* Updated the GIF in the readme

### Miscellaneous Tasks

- *(clippy)* Fixed clippy lints
## [0.1.8] - 2025-11-14

### Features

- *(TUI)* Added the ability to create files

### Bug Fixes

- *(clippy)* Removed `unwrap()`s and replaced with error propogation

### Documentation

- *(changelog)* Changed CHANGELOG format to include title
## [0.1.7] - 2025-11-13

### Features

- *(TUI)* Fixed effect timing
- *(TUI)* Changed Metadata pane, to include all metadata attributes in a nicer Table

### Bug Fixes

- *(bug)* Fixed state not resetting after directory download
- *(typo)* Fixed a type in the keybinds

### Documentation

- *(README)* Added features subsection to readme
## [0.1.6] - 2025-11-13

### Features

- *(SSH)* Added the feature to access an ssh session quickly from the browser.
## [0.1.5] - 2025-11-12

### Features

- *(TUI)* Updated keybind hints
- *(TUI)* Added the ability to edit files in an external editor.
## [0.1.4] - 2025-11-12

### Features

- *(TUI)* Added delete feature
- *(TUI)* Added ability to move files.
## [0.1.3] - 2025-11-12

### Documentation

- Added the GIF to the README
- *(README)* Added built with ratatui label
- *(README)* Added badges to the README
## [0.1.2] - 2025-11-12

### Features

- *(TUI)* Added file sizes to the download progress.

### Performance

- *(optimization)* Added optimization options to `Cargo.toml`

### Styling

- Formatted Changelog
## [0.1.1] - 2025-11-11

### Miscellaneous Tasks

- Removed useless print messages
## [0.1.0] - 2025-11-11

### Features

- Created till functioning tui
- *(TUI)* Made the TUI faster by removing multiple ssh connections
- *(TUI)* Added line gauge, fixed errors
- *(TUI)* Added a remaining files download list
- *(TUI)* Added TachyonFx for smooth transitions
- *(TUI)* Added content viewing
- [**breaking**] Added provisional README
- *(docs)* Added git-cliff support

### Bug Fixes

- *(TUI)* Fixed the tui-logger widget
- *(log)* Fixed logging
- *(lint)* Fixed clippy lints

### Refactor

- Separated tui.rs into separate module

### Documentation

- Committeed CHANGELOG.MD

### Miscellaneous Tasks

- *(release)* V0.1.0
