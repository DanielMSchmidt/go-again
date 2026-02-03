# go-again

A CLI tool that remembers your failing Go tests so you can re-run them quickly.

![go-again demo](demo.gif)

## The Problem

When working on a large Go codebase, you often:

1. Run all tests and see several failures
2. Pick one failing test to fix
3. Want to re-run *just* the failing tests to check your fix
4. Repeat until all tests pass

Manually copying test names and constructing `go test -run` commands gets tedious fast.

## The Solution

`go-again` automates this workflow:

```sh
# Run tests and remember failures
go test ./... | go-again remember

# Re-run only the failing tests
go-again run
```

That's it. Failed tests are stored per project and branch, so you can context-switch freely.

## Installation

### Homebrew (macOS) - Recommended

```sh
brew tap danielmschmidt/homebrew-tap
brew install go-again
```

Or in one command:

```sh
brew install danielmschmidt/homebrew-tap/go-again
```

### Binary Releases

Download pre-built binaries from [GitHub Releases](https://github.com/DanielMSchmidt/go-again/releases).

### From Source

Requires [Rust](https://rustup.rs/):

```sh
cargo install --git https://github.com/DanielMSchmidt/go-again
```

## Quick Start

```sh
# 1. Run your tests and pipe to go-again
go test ./... | go-again remember
# Output: Remembered 3 failing tests

# 2. Fix a bug, then re-run just the failing tests
go-again run

# 3. Once everything passes, clear the list
go-again clear
```

## Commands

### `remember` - Capture failing tests

Reads `go test` output from stdin and stores any failures.

```sh
go test ./... | go-again remember
go test ./... -v | go-again remember   # verbose works too
go test ./... -json | go-again remember # JSON output works too
```

### `run` - Re-run failing tests

Runs all remembered failing tests.

```sh
go-again run
```

Use `--update` to automatically remove tests that now pass:

```sh
go-again run --update
```

### `list` - Show failing tests

See what tests are currently remembered.

```sh
go-again list
# ./internal/logic TestCalculateSum
# ./api/handler TestHandleRequest
```

### `select` - Pick tests interactively

Opens an fzf-style picker to choose which tests to run.

```sh
go-again select
```

### `watch` - Interactive test loop

Combines `select` and `run` in a loop. Pick tests, run them, repeat.

```sh
go-again watch
```

### `clear` - Reset

Forget all remembered tests for the current project/branch.

```sh
go-again clear
```

## How It Works

- Failed tests are stored in `~/.go-again/state.json`
- Each project+branch combination has its own list
- Project identity is based on git repository root and current branch
- Switching branches automatically switches to that branch's failing tests

## Contributing

### Regenerating the demo GIF

The demo GIF is generated using [VHS](https://github.com/charmbracelet/vhs):

```sh
brew install vhs
vhs demo.tape
```

### Running the example project

The `testdata/example-project` contains a Go project with intentionally failing tests for CI verification:

```sh
cd testdata/example-project
go test ./... | go-again remember
go-again list
```

## License

MIT
