# Runtime Assets Directory

This directory contains pre-compiled WebAssembly runtimes for the Polyglot Runtime Adapter.

## Setup

Run the setup script to download runtimes:

### Windows (PowerShell):
```powershell
.\scripts\setup_runtimes.ps1
```

### Linux/macOS:
```bash
chmod +x scripts/setup_runtimes.sh
./scripts/setup_runtimes.sh
```

## Manual Download

If the setup script fails, manually download these files to `./assets/runtimes/`:

### PHP 8.2 CGI (`php-cgi.wasm`)
**Source**: VMWare Wasm Labs
**URL**: https://github.com/vmware-labs/webassembly-language-runtimes/releases

### Python 3.11 (`python.wasm`)
**Source**: VMWare Wasm Labs
**URL**: https://github.com/vmware-labs/webassembly-language-runtimes/releases

### Static Web Server (`static-server.wasm`)
**Source**: static-web-server project
**URL**: https://github.com/static-web-server/static-web-server/releases

## Usage

The Polyglot Runtime Adapter automatically detects the programming language of uploaded sites and injects the appropriate runtime:

- **PHP Sites** → `php-cgi.wasm`
- **Python Sites** → `python.wasm`
- **Static HTML** → `static-server.wasm`

No manual configuration needed!
