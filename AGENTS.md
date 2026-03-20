# Repository Guidelines

## Project Structure & Module Organization
`paracmd` is a small Rust CLI. Core logic lives in `src/main.rs`, including argument parsing, directory discovery, parallel execution, and unit tests. Package metadata is in `Cargo.toml`. CI and release automation live under `.github/workflows`, with helper scripts in `.github/scripts`. Generated artifacts go to `target/` and should not be committed.

## Build, Test, and Development Commands
Use the same commands enforced by CI:

- `cargo run -- --help`: run the CLI locally and inspect usage.
- `cargo build --release`: build an optimized binary in `target/release/`.
- `cargo test --locked`: run unit tests with the lockfile respected.
- `cargo fmt -- --check`: verify formatting.
- `cargo clippy --all-targets --all-features -- -D warnings`: treat lint warnings as errors.

Example manual run:

```powershell
cargo run -- D:\workspaces -- git status
```

## Coding Style & Naming Conventions
Follow standard Rust style: 4-space indentation, `snake_case` for functions and variables, `PascalCase` for structs and enums, and small focused helpers over deeply nested logic. Keep CLI messages explicit and platform-safe. Always run `cargo fmt` before opening a PR; CI also checks `clippy`, so avoid introducing warning suppressions unless they are justified in code comments.

## Testing Guidelines
Tests currently live inline in `src/main.rs` under `#[cfg(test)]`. Add focused unit tests alongside the code they exercise, using descriptive names like `parse_args_supports_separator_and_overrides`. Cover argument parsing, traversal depth, and failure cases for command execution. Run `cargo test --locked` before submitting changes.

## Commit & Pull Request Guidelines
Recent commits use short, imperative subjects such as `Add CI and automated GitHub release workflow`. Keep commit titles concise, capitalized, and action-oriented. PRs should summarize behavior changes, mention added or updated tests, and link related issues when applicable. For CLI output changes, include a short terminal transcript in the PR description.

## Release & Versioning Notes
Releases are tag-driven with tags like `v0.1.0`. Keep the version in `Cargo.toml` aligned with the release tag, and update `CHANGELOG.md` before cutting a release.
