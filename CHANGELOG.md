# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
## [unreleased]

### Features
- *(config)* Added a default config generator and fixed config detection (by @JayanAXHF)

## [0.3.0] - 2025-11-24

### Features
- *(TUI)* Added support for filtering out hidden files (by @JayanAXHF)


### Refactor
- *(state)* [**breaking**] Refactored MainUI's get_file_entries to be `CoW` to support hidden files filering (by @JayanAXHF)


### Styling
- *(file view)* Differentiated between directories and files using colour (by @JayanAXHF)

## [0.2.2] - 2025-11-22

### Features
- *(errors)* Added better error handling using panic-hooks and human-panic (by @JayanAXHF)


### Miscellaneous Tasks
- typo fixed (by @TimShilov)

## [0.2.1] - 2025-11-19

### Features
- *(build)* Added vergen to the build system (by @JayanAXHF)


### Refactor
- *(cli)* Refactored the CLI module to better fit the build system (by @JayanAXHF)


### Documentation
- *(README)* Updated usage instructions (by @JayanAXHF)

## [0.2.0] - 2025-11-19

### Features
- Added a config system to the project, allowing users to configure themes (by @JayanAXHF)
- *(docs)* Added man pages for the project and the config system (by @JayanAXHF)
- *(cli)* [**breaking**] Changed the CLI structure to be modular and enable subcommands like install-man-pages (by @JayanAXHF)
- Added clap-completion support (by @JayanAXHF)

## [0.1.10] - 2025-11-17

### Features
- *(TUI)* Added a display for the currect connection (by @JayanAXHF)


### Bug Fixes
- *(TUI)* Fixed inconsistent border colours, and removed unnecessary whitespace (by @JayanAXHF)


### Other
- *(docs)* Merge pull request #18 from ZennoZenith/patch-1 (by @JayanAXHF)


### Documentation
- Update the README to include license information and added the relevant license files. (by @JayanAXHF)
- *(README)* Updated the GIF in the readme (by @JayanAXHF)


### Miscellaneous Tasks
- *(clippy)* Fixed clippy lints (by @JayanAXHF)

## [0.1.8] - 2025-11-14

### Features
- *(TUI)* Added the ability to create files (by @JayanAXHF)


### Bug Fixes
- *(clippy)* removed `unwrap()`s and replaced with error propogation (by @JayanAXHF)


### Documentation
- *(changelog)* Changed CHANGELOG format to include title (by @JayanAXHF)

## [0.1.7] - 2025-11-13

### Features
- *(TUI)* Fixed effect timing (by @JayanAXHF)
- *(TUI)* Changed Metadata pane, to include all metadata attributes in a nicer Table (by @JayanAXHF)


### Bug Fixes
- *(bug)* fixed state not resetting after directory download (by @JayanAXHF)
- *(typo)* Fixed a type in the keybinds (by @JayanAXHF)


### Documentation
- *(README)* Added features subsection to readme (by @JayanAXHF)

## [0.1.6] - 2025-11-13

### Features
- *(SSH)* Added the feature to access an ssh session quickly from the browser. (by @JayanAXHF)

## [0.1.5] - 2025-11-12

### Features
- *(TUI)* Updated keybind hints (by @JayanAXHF)
- *(TUI)* Added the ability to edit files in an external editor. (by @JayanAXHF)

## [0.1.4] - 2025-11-12

### Features
- *(TUI)* Added delete feature (by @JayanAXHF)
- *(TUI)* Added ability to move files. (by @JayanAXHF)

## [0.1.3] - 2025-11-12

### Documentation
- Added the GIF to the README (by @JayanAXHF)
- *(README)* Added built with ratatui label (by @JayanAXHF)
- *(README)* Added badges to the README (by @JayanAXHF)

## [0.1.2] - 2025-11-12

### Features
- *(TUI)* Added file sizes to the download progress. (by @JayanAXHF)


### Performance
- *(optimization)* Added optimization options to `Cargo.toml` (by @JayanAXHF)


### Styling
- Formatted Changelog (by @JayanAXHF)

## [0.1.1] - 2025-11-11

### Miscellaneous Tasks
- removed useless print messages (by @JayanAXHF)

## [0.1.0] - 2025-11-11

### Features
- Created till functioning tui (by @JayanAXHF)
- *(TUI)* Made the TUI faster by removing multiple ssh connections (by @JayanAXHF)
- *(TUI)* Added line gauge, fixed errors (by @JayanAXHF)
- *(TUI)* Added a remaining files download list (by @JayanAXHF)
- *(TUI)* Added TachyonFx for smooth transitions (by @JayanAXHF)
- *(TUI)* Added content viewing (by @JayanAXHF)
- [**breaking**] Added provisional README (by @JayanAXHF)
- *(docs)* Added git-cliff support (by @JayanAXHF)


### Bug Fixes
- *(TUI)* fixed the tui-logger widget (by @JayanAXHF)
- *(log)* Fixed logging (by @JayanAXHF)
- *(lint)* Fixed clippy lints (by @JayanAXHF)


### Refactor
- Separated tui.rs into separate module (by @JayanAXHF)


### Documentation
- Committeed CHANGELOG.MD (by @JayanAXHF)


### Miscellaneous Tasks
- *(release)* v0.1.0 (by @JayanAXHF)
