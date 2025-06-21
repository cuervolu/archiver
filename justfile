# --- Aliases and Settings ---

# For commands like `just r my-project`
alias r := restore
alias ra := restore-all
alias l := list
alias a := run
alias d := delete
alias e := exclude

# --- Development & CI Workflow ---

# The default command is executed with `just`.
default:
    @just --list

# Run all tests of the workspace
test: test-core test-cli
    @echo "✅ All tests passed!"

# Run all tests in the core package
test-core:
    @echo "🧪 Running core tests..."
    @cargo test --package archiver-core -- --nocapture

# Run the CLI tests
test-cli:
    @echo "🧪 Running CLI tests..."
    @cargo test --package archiver-cli --test cli_tests

# Execute clippy to lint the codebase
lint:
    @cargo clippy --workspace --all-targets --all-features -- -D warnings

# Format the codebase using `cargo fmt`
fmt:
    @cargo fmt --all

# Faster than `cargo build` for checking the code
check:
    @cargo check --workspace

# Compile the project in debug mode
build:
    @cargo build --workspace

# Compile the project in release mode
build-release:
    @cargo build --workspace --release


# --- Running the Application ---

# Install the binary in the cargo PATH to be able to use `archive` globally.
install: build-release
    @cargo install --path crates/archiver-cli

# Execute the interactive initialization command
init:
    @cargo run --package archiver-cli -- init

# Execute the interactive configuration command
config:
    @cargo run --package archiver-cli -- config

# Restore a specific archived project
# Example: `just restore my-project`
restore NAME:
    @cargo run --package archiver-cli -- restore {{NAME}}

# Restore all archived projects
restore-all:
    @cargo run --package archiver-cli -- restore --all

# Scans and archives inactive projects
# Add the --dry-run flag with: `just run --dry-run`.
run *ARGS:
    @cargo run --package archiver-cli -- run {{ARGS}}

# List all archived projects
list:
    @cargo run --package archiver-cli -- list

# Add or remove a project from the exclusion list
# Example: `just exclude my-project` or `just exclude --remove my-project`
exclude *ARGS:
    @cargo run --package archiver-cli -- exclude {{ARGS}}

# Delete an archived project permanently
# Example: `just delete my-project` or `just delete --all`
delete *ARGS:
    @cargo run --package archiver-cli -- delete {{ARGS}}
