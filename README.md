# Commandy

> Secure, fast command suggestions using local models

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org/)

Translates natural language into shell commands using local AI models via llama.cpp with Gemma 3 270M.

## Quick Start

### Installation

**Supported Platforms:**
- macOS (Intel & Apple Silicon)
- Linux (x86_64) - Debian, Ubuntu, and other distributions

```bash
# Quick install (recommended)
curl -fsSL https://raw.githubusercontent.com/aptro/commandy/main/install.sh | sh

# OR download from releases
# https://github.com/aptro/commandy/releases

# Initialize Commandy
commandy init
```

### Basic Usage
```bash
# Natural language to commands
commandy "list running containers"
commandy "find files larger than 100MB"
commandy "git commit with message hello world"

# With explanations
commandy --explain "compress this directory"

# Validates real executables
commandy "memgraph query to get all nodes"
# ✅ Suggests: cypher-shell -a bolt://localhost:7687 "MATCH (n) RETURN n"
# ❌ Not: memgraph query "..." (checks with 'which' first)
```

### Interactive Controls
- **Enter** → Execute command immediately
- **Tab** → Copy to clipboard  
- **Escape** → Modify/follow-up on command
- **Escape Escape** → Exit to static view
- **F** → Alternative follow-up key

## How It Works

### Caching
```bash
# First few uses: AI generates fresh suggestions
commandy "docker logs for container"

# After 5+ successful uses with >70% success rate:
# → Instantly returns cached: docker logs <container_name>

# View cache statistics
commandy config
```

### Learning
Commandy evolves with your usage through `~/.commandy/COMMANDY.md`:

```markdown
### Docker
Last updated: 2024-01-24
✓ Validated executable: `docker`
Context: "list running containers"  
Full command: `docker ps -a --format "table {{.Names}}\t{{.Status}}"`

✓ Successful execution:
"docker logs for container" → `docker logs my-app`
```

### AI Model
- **Gemma 3 270M**: Ultra-compact 270 million parameter model (292MB)
- **Local inference**: Runs entirely offline via llama.cpp binary
- **Cross-platform**: Native binaries for macOS (Intel & ARM64) and Linux
- **Efficient**: Extremely low resource usage and battery consumption
- **No services**: No background processes or HTTP servers needed

### Validation
- Validates commands using `which` and system PATH
- Scans `/usr/local/bin`, `/usr/bin`, `/bin` for available tools
- Rejects pseudo-commands and API-style syntax
- Learns valid executables progressively

## Commands

```bash
commandy init                    # Initialize setup
commandy config                  # Show configuration & cache stats
commandy doctor                  # Run diagnostics  
commandy clear --cache          # Clear suggestion cache
commandy clear --context        # Reset learning context
commandy "your natural language query"
```

## Project Structure

```
~/.commandy/
├── COMMANDY.md              # Evolving knowledge base
├── config.toml              # Configuration
├── bin/                     # llama.cpp binary
├── cache/
│   └── suggestions.db       # Smart cache with success tracking
└── backups/                 # COMMANDY.md backups

src/
├── cli/                     # Command-line interface & interactions  
├── ai/                      # llama.cpp integration & prompt engineering
├── context/                 # Caching, learning, shell history
├── config/                  # Configuration management
└── utils/                   # Environment detection, validation
```

## Development Setup

For contributors and developers who want to build locally:

```bash
# Clone the repository
# SSH (if you have GitHub SSH keys set up)
git clone git@github.com:aptro/commandy.git
# OR HTTPS
git clone https://github.com/aptro/commandy.git
cd commandy

# Build and install
cargo build --release
cargo install --path .

# Initialize for development
commandy init
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution guidelines.

## License

MIT License - see [LICENSE](LICENSE) for details.