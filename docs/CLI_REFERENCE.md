# Pear Server CLI Reference

Complete reference guide for the Pear Server Command Line Interface.

## Installation

```bash
# Build from source
cargo build --release

# Binary will be at: target/release/pear
# Add to PATH for system-wide access
```

## Global Options

All commands support these global flags:

| Flag | Description |
|------|-------------|
| `-h, --help` | Show help information |
| `-V, --version` | Show version information |

## Commands

### `pear start`

Start the Pear Server daemon.

**Usage:**
```bash
pear start [OPTIONS]
```

**Options:**
| Flag | Description | Default |
|------|-------------|---------|
| `-c, --config <FILE>` | Configuration file path | `pear.toml` |
| `-f, --foreground` | Run in foreground (don't daemonize) | false |
| `-v, --verbose` | Enable verbose logging | false |

**Examples:**
```bash
# Start with default configuration
pear start

# Start with custom config
pear start --config production.toml

# Run in foreground with verbose logging
pear start --foreground --verbose
```

---

### `pear stop`

Stop the running Pear Server.

**Usage:**
```bash
pear stop [OPTIONS]
```

**Options:**
| Flag | Description | Default |
|------|-------------|---------|
| `-f, --force` | Force shutdown without graceful period | false |

**Examples:**
```bash
# Graceful shutdown
pear stop

# Force immediate shutdown
pear stop --force
```

---

### `pear status`

Show server status and statistics.

**Usage:**
```bash
pear status [OPTIONS]
```

**Options:**
|Flag | Description | Default |
|------|-------------|---------|
| `-f, --format <FORMAT>` | Output format: text, json, or table | `table` |

**Examples:**
```bash
# Show status as table (default)
pear status

# Get JSON output for parsing
pear status --format json

# Simple text output
pear status --format text
```

**Sample Output (table):**
```
┌─── Pear Server Status ─────────────────────────────┐
│                                                     │
│  Status: RUNNING                                    │
│  Version: 0.3.0                                     │
│  Uptime: 1h 30m                                     │
│                                                     │
│  Cage Pools ───────────────────                    │
│    Total Cages: 3  Healthy: 3  Crashed: 0          │
│                                                     │
│  Traffic Stats ─────────────────                   │
│    Total Requests: 156,432                          │
│    Requests/sec: 42.3                               │
│                                                     │
│  Resources ────────────────────                    │
│    Memory Usage: 287 MB                             │
│    CPU Usage: 23%                                   │
│                                                     │
└─────────────────────────────────────────────────────┘
```

---

### `pear deploy`

Deploy a WebAssembly module to a site.

**Usage:**
```bash
pear deploy <WASM_FILE> [OPTIONS]
```

**Arguments:**
| Argument | Description | Required |
|----------|-------------|----------|
| `<WASM_FILE>` | Path to .wasm file | Yes |

**Options:**
| Flag | Description | Default |
|------|-------------|---------|
| `-s, --site <SITE>` | Site identifier | `default-site` |
| `-r, --replicas <N>` | Number of Cage replicas | `3` |

**Examples:**
```bash
# Deploy with default settings
pear deploy my-app.wasm

# Deploy to specific site with 5 replicas
pear deploy my-app.wasm --site production --replicas 5
```

---

### `pear config`

Manage server configuration.

**Usage:**
```bash
pear config <SUBCOMMAND>
```

**Subcommands:**

#### `pear config show`

Show current configuration.

```bash
pear config show
```

**Output:**
```
[server]
  http2_port = 8080
  http3_port = 8443
  dashboard_port = 9000

[cages]
  default_replicas = 3
  memory_limit_mb = 128
  cpu_timeout_ms = 1000

[ai]
  enable_anomaly_detection = true
  anomaly_threshold = 0.8
```

#### `pear config set`

Set a configuration value.

```bash
pear config set <KEY> <VALUE>
```

**Examples:**
```bash
# Change HTTP/2 port
pear config set server.http2_port 9080

# Increase Cage memory limit
pear config set cages.memory_limit_mb 256
```

#### `pear config validate`

Validate configuration file.

```bash
pear config validate [--file <FILE>]
```

**Examples:**
```bash
# Validate default pear.toml
pear config validate

# Validate specific file
pear config validate --file production.toml
```

---

### `pear dashboard`

Show dashboard URL and access information.

**Usage:**
```bash
pear dashboard
```

**Output:**
```
📊 Pear Server Dashboard

  URL: http://localhost:9000
  Status: Available

Features:
  • Real-time Cage Pool visualization
  • Live traffic statistics
  • AI security threat alerts
  • Request logs streaming
  • Configuration management
```

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | General error |
| `2` | Configuration error |
| `130` | Interrupted by user (Ctrl+C) |

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `RUST_LOG` | Log level filter | `info` |
| `PEAR_CONFIG` | Override config file path | `pear.toml` |

**Example:**
```bash
# Enable debug logging
RUST_LOG=debug pear start

# Use alternative config
PEAR_CONFIG=/etc/pear/config.toml pear start
```

## Configuration File

See [pear.toml.example](../pear.toml.example) for a complete configuration template.

Key sections:
- `[server]` - Network ports and binding
- `[ssl]` - SSL/TLS and ACME settings
- `[cages]` - WebAssembly Cage configuration
- `[ai]` - AI security module settings
- `[dashboard]` - Dashboard server configuration

## Troubleshooting

### Server won't start

```bash
# Check configuration
pear config validate

# Enable verbose logging
pear start --verbose
```

### Permission denied

```bash
# On Linux, ports <1024 require root
sudo pear start

# Or use higher ports in configuration
pear config set server.http2_port 8080
```

###  Port already in use

```bash
# Check what's using the port
netstat -tulpn | grep :8080

# Change port in configuration
pear config set server.http2_port 9080
```

## Quick Start

```bash
# 1. Create configuration (optional)
cp pear.toml.example pear.toml

# 2. Start server
pear start

# 3. Check status
pear status

# 4. Deploy your app
pear deploy my-app.wasm

# 5. Access dashboard
# Open http://localhost:9000 in browser

# 6. Stop server
pear stop
```

---

**For more information, visit the [Pear Server Documentation](../README.md)**
