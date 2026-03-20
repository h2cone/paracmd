# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-03-20

### Added

- Parallel command execution across subdirectories with a configurable worker count.
- Recursive directory discovery with a configurable maximum depth.
- Per-directory stdout, stderr, and exit status reporting in the final summary.
- CLI parsing that supports both implicit commands and an explicit `--` separator.
- Unit tests covering argument parsing and directory traversal depth behavior.
