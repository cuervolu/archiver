# Archive
![Status: In Development](https://img.shields.io/badge/Status-In_Development-orange.svg)

A command-line tool to automatically find and archive inactive local projects, keeping your workspace clean and tidy.

## Platform Support
Currently, **Archive** is developed and tested primarily for **Linux**. Support for other platforms like Windows or macOS is not yet available.

## Usage Examples

1.  **Initialize Configuration (First-time use)**
    Run the interactive setup to create your configuration file.
    ```bash
    archive init
    ```

2.  **Preview What Will Be Archived**
    Get a safe preview of which projects are considered inactive without moving any files.
    ```bash
    archive run --dry-run
    ```

3.  **Run the Archiving Process**
    Move all inactive projects to your archive directory.
    ```bash
    archive run
    ```

4.  **List Archived Projects**
    See a list of everything you've archived.
    ```bash
    archive list
    ```

5.  **Restore a Project**
    Bring a project back from the archive to its original location.
    ```bash
    archive restore "my-old-project"
    ```

## Current Features

➤ **Smart Scanning:** Detects project activity for both Git repositories (based on the last commit across all branches) and regular directories (based on the last file modification time).

➤ **Archive & Restore:** Safely moves inactive projects to a dedicated directory and allows you to restore them easily.

➤ **Interactive Setup:** An `init` command guides you through creating your configuration file for the first time.

➤ **Dry Run Mode:** The `archive --dry-run` command allows you to preview which projects would be archived without making any changes.

➤ **Configurable Logging:** Adjust log verbosity using `-v` for debug and `-vv` for trace details.

## TODO

- [ ] **Complete CLI Functionality**
    - [ ] Implement project cleanup rules (e.g., deleting `node_modules`, `target/`).
    - [ ] Add auto-delete feature for projects archived for a long time.
    - [ ] Implement desktop notifications for completed actions.

- [ ] **Terminal User Interface (TUI)**
    - [ ] Build an interactive TUI with `ratatui` for a visual way to manage archived projects.
    - [ ] Add features for listing, restoring, and searching within the TUI.

- [ ] **Background Automation**
    - [ ] Provide `systemd` service and timer files for automatic execution on Linux.

- [ ] **Documentation**
    - [ ] Write comprehensive user documentation for all commands and configuration options.
    - [ ] Add more examples to the `justfile`.

- [ ] **Publishing**
    - [ ] Prepare and publish the `archive-core` and `archive` crates to crates.io.