# paracmd

Run the same shell command across subdirectories in parallel.

## Features

- Scans subdirectories under a root path up to a configurable depth
- Runs a command in each matched directory with a configurable worker count
- Preserves per-directory stdout, stderr, and exit status in the final report
- Works with or without an explicit `--` separator before the command

## Getting Started

### Prerequisites

- Rust toolchain with Cargo installed

### Build

```powershell
cargo build --release
```

### Run

```powershell
cargo run -- D:\workspaces -- git status
```

With explicit depth and job settings:

```powershell
cargo run -- --depth 2 --jobs 8 D:\workspaces -- cargo test
```

### Test

```powershell
cargo test
```

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE).
