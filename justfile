# --- Aliases and Settings ---

# For commands like `just r my-project`
alias r := restore
alias l := list
alias a := archive
alias e := exclude

# --- Development & CI Workflow ---

# The default command is executed with `just`.
default:
    @just --list

# Run all tests of the workspace
test: test-core test-cli
    @echo "✅ All tests passed!"

# Run the tests in the core package
test-core:
    @echo "🧪 Running core tests..."
    @cargo test --package archiver-core --test archive_process -- --nocapture

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

# Install the binary in the cargo PATH to be able to use `archiver` globally.
install: build-release
    @cargo install --path crates/archiver-cli

# Execute the interactive initialization command
init:
    @cargo run --package archiver-cli -- init

# Execute the interactive configuration command
config:
    @cargo run --package archiver-cli -- config

# Restore an archived project
# Example: `just restore my-project`
restore NAME:
    @cargo run --package archiver-cli -- restore {{NAME}}

# Archive inactive projects
# Add the --dry-run flag with: `just archive -- --dry-run`.
archive *ARGS:
    @cargo run --package archiver-cli -- archive {{ARGS}}

# List all archived projects
list:
    @cargo run --package archiver-cli -- list

# Add or remove a project from the exclusion list
# Example: `just exclude my-project` or `just exclude --remove my-project`
exclude *ARGS:
    @cargo run --package archiver-cli  -- exclude {{ARGS}}