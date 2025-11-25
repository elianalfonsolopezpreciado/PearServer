# Pear Server Runtime Asset Manager
# Downloads pre-compiled WebAssembly runtimes for polyglot support (Windows PowerShell)

Write-Host "========================================" -ForegroundColor Blue
Write-Host "🍐 Pear Server Runtime Setup" -ForegroundColor Blue
Write-Host "========================================" -ForegroundColor Blue
Write-Host ""

# Configuration
$RUNTIME_DIR = ".\assets\runtimes"
$TMP_DIR = "$env:TEMP\pear-runtimes"

# Create directories
Write-Host "Creating runtime directories..." -ForegroundColor Green
New-Item -ItemType Directory -Force -Path $RUNTIME_DIR | Out-Null
New-Item -ItemType Directory -Force -Path $TMP_DIR | Out-Null

# Function to download runtime
function Download-Runtime {
    param(
        [string]$Name,
        [string]$Url,
        [string]$Output
    )
    
    Write-Host "Downloading $Name..." -ForegroundColor Blue
    
    try {
        $OutFile = Join-Path $TMP_DIR $Output
        Invoke-WebRequest -Uri $Url -OutFile $OutFile -UseBasicParsing
        Write-Host "✓ Downloaded $Name" -ForegroundColor Green
        return $true
    }
    catch {
        Write-Host "✗ Failed to download $Name" -ForegroundColor Red
        Write-Host "Error: $_" -ForegroundColor Red
        return $false
    }
}

# 1. PHP CGI Runtime
Write-Host ""
Write-Host "[1/3] PHP Runtime" -ForegroundColor Yellow
$PHP_URL = "https://github.com/vmware-labs/webassembly-language-runtimes/releases/download/php%2F8.2.0%2B20230707-1755149/php-cgi-8.2.0.wasm"
Download-Runtime "PHP 8.2 CGI" $PHP_URL "php-cgi.wasm"

# 2. Python Runtime
Write-Host ""
Write-Host "[2/3] Python Runtime" -ForegroundColor Yellow
$PYTHON_URL = "https://github.com/vmware-labs/webassembly-language-runtimes/releases/download/python%2F3.11.3%2B20230428-173305/python-3.11.3.wasm"
Download-Runtime "Python 3.11" $PYTHON_URL "python.wasm"

# 3. Static Web Server
Write-Host ""
Write-Host "[3/3] Static Web Server" -ForegroundColor Yellow
$STATIC_URL = "https://github.com/static-web-server/static-web-server/releases/download/v2.24.2/static-web-server-v2.24.2-x86_64-pc-windows-msvc-wasm32-wasi.wasm"
Download-Runtime "Static Web Server" $STATIC_URL "static-server.wasm"

# Move to runtime directory
Write-Host ""
Write-Host "Installing runtimes..." -ForegroundColor Green
Copy-Item -Path "$TMP_DIR\*.wasm" -Destination $RUNTIME_DIR -Force -ErrorAction SilentlyContinue

# Verify installations
Write-Host ""
Write-Host "========================================" -ForegroundColor Blue
Write-Host "✓ Setup Complete" -ForegroundColor Blue
Write-Host "========================================" -ForegroundColor Blue
Write-Host ""
Write-Host "Runtime directory: $RUNTIME_DIR"
Write-Host ""
Write-Host "Installed runtimes:"

$phpPath = Join-Path $RUNTIME_DIR "php-cgi.wasm"
if (Test-Path $phpPath) {
    $phpSize = (Get-Item $phpPath).Length / 1MB
    Write-Host "✓ PHP 8.2 CGI ($([math]::Round($phpSize, 2)) MB)" -ForegroundColor Green
} else {
    Write-Host "✗ PHP runtime missing" -ForegroundColor Red
}

$pyPath = Join-Path $RUNTIME_DIR "python.wasm"
if (Test-Path $pyPath) {
    $pySize = (Get-Item $pyPath).Length / 1MB
    Write-Host "✓ Python 3.11 ($([math]::Round($pySize, 2)) MB)" -ForegroundColor Green
} else {
    Write-Host "✗ Python runtime missing" -ForegroundColor Red
}

$staticPath = Join-Path $RUNTIME_DIR "static-server.wasm"
if (Test-Path $staticPath) {
    $staticSize = (Get-Item $staticPath).Length /  1MB
    Write-Host "✓ Static Web Server ($([math]::Round($staticSize, 2)) MB)" -ForegroundColor Green
} else {
    Write-Host "✗ Static runtime missing" -ForegroundColor Red
}

Write-Host ""
Write-Host "Note: These are pre-compiled WebAssembly binaries from trusted sources:" -ForegroundColor Blue
Write-Host "  - PHP & Python: VMWare Wasm Labs"
Write-Host "  - Static Server: static-web-server project"
Write-Host ""
Write-Host "You can now run: cargo build --release" -ForegroundColor Green

# Cleanup
Remove-Item -Path $TMP_DIR -Recurse -Force -ErrorAction SilentlyContinue
