# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

- `cargo build` - Build the project
- `cargo run` - Build and run the binary
- `cargo test` - Run all tests
- `cargo test <test_name>` - Run a specific test
- `cargo clippy` - Run linter
- `cargo fmt` - Format code

## Project Overview

`go-again` is a Rust CLI that remembers failing Go tests and re-runs them. The workflow:

1. `go test ./... | go-again remember` - Parse test output, store failures
2. `go-again run` - Re-run only the failing tests
3. `go-again clear` - Reset when done

State is stored in `~/.go-again/state.json`, keyed by git repo + branch.

## Commands

| Command | Description |
|---------|-------------|
| `remember` | Read stdin, store failing tests |
| `run` | Re-run stored failing tests (`--update` removes passing tests) |
| `list` | Show stored failing tests |
| `select` | fzf-style picker to choose tests |
| `watch` | Interactive select + run loop |
| `clear` | Remove all stored tests for current project |

## Key Files

- `src/parser.rs` - Parses `go test` output
- `src/storage.rs` - State persistence (`GO_AGAIN_STATE_DIR` env var for testing)
- `src/runner.rs` - Executes `go test` commands
- `src/project.rs` - Git repo/branch detection
- `tests/cli_tests.rs` - Integration tests
