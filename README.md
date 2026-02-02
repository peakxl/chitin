# Chitin

A fast Rust shell for 🦞[OpenClaw](https://docs.openclaw.ai) — the exoskeleton that makes it snappy.

## Why Chitin?

The OpenClaw CLI is built on Node.js, which means every command pays a ~3 second startup tax. This makes shell completion painfully slow and `--help` feel sluggish.

Chitin wraps the Node.js CLI with a native Rust binary that:
- Serves `--help` and `--version` instantly from cache (main + all subcommands)
- Delegates all other commands to the real OpenClaw CLI
- Rebrands output so you see `chitin` in usage examples

The name "chitin" refers to the material that forms crab claws and exoskeletons — a fitting name for the fast, native shell around OpenClaw.

## Installation

### From Source

```bash
git clone https://github.com/peakxl/chitin
cd chitin
cargo build --release
```

Copy the binary to your PATH:

```bash
cp target/release/chitin ~/.local/bin/
# or
sudo cp target/release/chitin /usr/local/bin/
```

### Prerequisites

Chitin requires a Node.js runtime (v22+) and the OpenClaw CLI for full functionality.

If these aren't installed, running `chitin` will offer to install them for you:
- Installs pnpm (recommended) or uses existing npm
- Installs Node.js 22 via pnpm
- Installs the OpenClaw CLI globally

Or install manually:

```bash
# Using pnpm (recommended)
pnpm add -g openclaw@latest

# Using npm
npm install -g openclaw@latest
```

If OpenClaw isn't installed, running `chitin` will guide you through the installation process.

## Usage

Use `chitin` exactly like you would use `openclaw`:

```bash
chitin --help              # Instant help (cached)
chitin --version           # Shows both chitin and openclaw versions
chitin gateway             # Delegates to openclaw gateway
chitin channels login      # Delegates to openclaw channels login
chitin agent --to +1...    # Delegates to openclaw agent
```

## How It Works

```
┌─────────────────────────────────────────────────────────┐
│                        chitin                           │
│                    (Rust binary)                        │
├─────────────────────────────────────────────────────────┤
│  --help, --version    │    All other commands           │
│         ↓             │            ↓                    │
│   Cache lookup        │   Delegate to Node.js           │
│   (~/.chitin/cache)   │   openclaw CLI                  │
│         ↓             │            ↓                    │
│   2ms response        │   Full functionality            │
└─────────────────────────────────────────────────────────┘
```

### Caching

- First run fetches help from Node.js (~3s), subsequent runs are instant (~2ms)
- Help for subcommands is cached on first use
- Cache invalidates when OpenClaw or Chitin version changes, or after 24 hours
- Cache location: `~/.chitin/cache/help_cache.json`

## Planned Features

These features would benefit from community contributions:

### Shell Completion (Help Wanted)

Generate completion scripts natively in Rust for instant tab completion. Currently shell completion still goes through Node.js and is slow.

**What's needed:**
- Parse OpenClaw's command structure
- Generate completions for bash, zsh, fish
- Cache completion data alongside help

### Native Command Implementation (Help Wanted)

Reimplement frequently-used commands in Rust for instant response:

**Candidates:**
- `chitin status` — Quick health check
- `chitin health` — Gateway health probe
- `chitin config get/set` — Config file operations
- `chitin sessions` — List sessions from local storage

**What's needed:**
- Identify which commands are most used
- Implement without breaking compatibility
- Maintain feature parity with Node.js version

### Cross-Platform Binaries (Help Wanted)

Provide pre-built binaries for common platforms:
- Linux x86_64 / arm64
- macOS x86_64 / arm64 (Apple Silicon)
- Windows x86_64

**What's needed:**
- CI/CD pipeline (GitHub Actions)
- Release automation
- Platform-specific testing

## Building

```bash
# Debug build
cargo build

# Release build (optimized, ~832 KB)
cargo build --release

# Run tests
cargo test

# Format and lint
cargo fmt
cargo clippy
```

## Project Structure

```
chitin/
├── Cargo.toml          # Project configuration
├── Cargo.lock          # Locked dependencies
└── src/
    ├── main.rs         # CLI entry point, help caching, delegation
    ├── cache.rs        # Help cache management
    ├── runtime.rs      # Node/npm/pnpm detection
    └── installer.rs    # Interactive installation flow
```

## License

MIT
