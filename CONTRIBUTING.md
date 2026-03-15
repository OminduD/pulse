# Contributing to Pulse

First off, thank you for considering contributing to Pulse! It's people like you that make Pulse such a great tool.

## Getting Started

1. **Fork the repository** on GitHub.
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/pulse.git
   cd pulse
   ```
3. **Set up the upstream remote**:
   ```bash
   git remote add upstream https://github.com/OminduD/pulse.git
   ```

## Development Environment

Pulse is written in Rust. You will need the Rust toolchain installed. We recommend using [rustup](https://rustup.rs/).

To build the project:
```bash
cargo build
```

To run the project:
```bash
cargo run
```

## Making Changes

1. **Create a new branch** for your feature or bug fix:
   ```bash
   git checkout -b feature/my-new-feature
   ```
2. **Make your changes**. 
3. **Format your code** using `rustfmt`:
   ```bash
   cargo fmt
   ```
4. **Lint your code** using `clippy` to ensure optimal Rust practices:
   ```bash
   cargo clippy -- -D warnings
   ```
5. **Run the tests** to make sure everything is working as expected:
   ```bash
   cargo test
   ```

## Submitting a Pull Request

1. Commit your changes with a clear and concise commit message.
2. Push your branch to your fork:
   ```bash
   git push origin feature/my-new-feature
   ```
3. Open a Pull Request on the main Pulse repository.
4. Provide a detailed description of the changes you've made in the PR description. If it fixes an open issue, please reference the issue number (e.g., `Fixes #123`).

## Reporting Issues

If you find a bug or have a feature request, please create an issue on GitHub. When reporting a bug, please include:
* Your operating system and version.
* The version of Pulse you are using.
* Steps to reproduce the issue.
* Any relevant error messages or logs.

Thank you for contributing!
