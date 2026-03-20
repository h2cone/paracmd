# paracmd

Run the same shell command across subdirectories in parallel.

## Features

- Scans subdirectories under a root path up to a configurable depth
- Runs a command in each matched directory with a configurable worker count
- Preserves per-directory stdout, stderr, and exit status in the final report
- Works with or without an explicit `--` separator before the command

## Getting Started

### Install

Download a prebuilt binary from [GitHub Releases](https://github.com/h2cone/paracmd/releases/).

Optional: put the binary in your `PATH` and run:

```powershell
paracmd --help
```

### Usage

```text
paracmd [--depth N] [--jobs N] <directory> -- <command> [args...]
paracmd [--depth N] [--jobs N] <directory> <command> [args...]
```

`paracmd` scans subdirectories under `<directory>` and runs the command in each matched subdirectory. The root directory itself is not included.

Options:

- `-d`, `--depth <N>`: maximum scan depth, default `1`
- `-j`, `--jobs <N>`: worker count, default = available CPU parallelism
- `-h`, `--help`: show help

Examples:

```powershell
paracmd D:\workspaces -- git status
paracmd --depth 2 --jobs 8 D:\workspaces -- cargo test
```

If any command fails, `paracmd` exits with a non-zero status after printing the per-directory results.

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE).
