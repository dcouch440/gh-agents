# Installation

## Prerequisites

- **Rust 1.75+** - Install via [rustup](https://rustup.rs/)
- **SQLite 3** - Usually pre-installed on macOS/Linux
- **Git** - For cloning and git operations

### API Keys

You'll need at least one of these:
- **Anthropic API Key** - Required for AI agents ([Get one](https://console.anthropic.com/))
- **GitHub Token** - Optional, for GitHub integration ([Create token](https://github.com/settings/tokens))

## Quick Install

### From Source (Recommended)

```bash
# Clone the repository
git clone https://github.com/your-org/nexor.git
cd nexor

# Build and install
cargo install --path .

# Verify installation
nexor --help
```

### Via Cargo

```bash
cargo install nexor
```

### Via Docker

```bash
# Build locally
docker build -t nexor -f docker/Dockerfile .

# Run with TUI
docker run -it -e ANTHROPIC_API_KEY=$ANTHROPIC_API_KEY -v $(pwd):/project nexor
```

See [Docker documentation](./docker.md) for full details.

## Platform-Specific Notes

### macOS

SQLite and Git are pre-installed. Just install Rust:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Linux (Ubuntu/Debian)

```bash
# Install dependencies
sudo apt update
sudo apt install build-essential pkg-config libssl-dev sqlite3

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Linux (Fedora/RHEL)

```bash
sudo dnf install gcc pkg-config openssl-devel sqlite
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Windows

1. Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/)
2. Install Rust via [rustup-init.exe](https://win.rustup.rs/)
3. SQLite is bundled with the build

## Setting Up API Keys

Create a `.env` file in your project or set environment variables:

```bash
# Option 1: .env file
echo 'ANTHROPIC_API_KEY=sk-ant-api03-...' >> .env
echo 'GITHUB_TOKEN=ghp_...' >> .env

# Option 2: Environment variables
export ANTHROPIC_API_KEY=sk-ant-api03-...
export GITHUB_TOKEN=ghp_...
```

## Verifying Installation

```bash
# Check version
nexor --version

# Run with help
nexor --help

# Start TUI (should show logo)
nexor
```

If you see the nexor logo, you're ready to go!

## Troubleshooting Installation

### "command not found: nexor"

Add cargo bin to PATH:
```bash
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

### "error: linker not found"

Install build tools:
- macOS: `xcode-select --install`
- Linux: `sudo apt install build-essential`

### "openssl not found"

Install OpenSSL dev package:
- Ubuntu: `sudo apt install libssl-dev`
- Fedora: `sudo dnf install openssl-devel`
- macOS: `brew install openssl`

### "API key not found"

Ensure the environment variable is set:
```bash
echo $ANTHROPIC_API_KEY  # Should show your key
```

If empty, set it:
```bash
export ANTHROPIC_API_KEY=your-key-here
```

## Next Steps

- [Configuration Guide](./configuration.md) - Customize nexor settings
- [Usage Guide](./usage.md) - Learn how to use nexor
- [Command Reference](./commands.md) - All available commands
