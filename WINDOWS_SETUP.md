# Pear Server Phase 1 - Setup Guide for Windows

This guide helps you set up the Rust development environment on Windows and build Pear Server.

## Step 1: Install Rust

1. **Download Rust installer**
   - Visit: https://rustup.rs/
   - Download `rustup-init.exe`

2. **Run the installer**
   - Double-click `rustup-init.exe`
   - Follow the prompts (default installation is recommended)
   - This will install:
     - `rustup` (Rust toolchain manager)
     - `rustc` (Rust compiler)
     - `cargo` (Rust package manager)

3. **Restart your terminal/PowerShell**
   - After installation, close and reopen PowerShell/Terminal
   - This ensures the PATH is updated

4. **Verify installation**
   ```powershell
   cargo --version
   rustc --version
   ```

## Step 2: Build Pear Server

1. **Navigate to the project directory**
   ```powershell
   cd "C:\Users\Elian\Desktop\Escritorio\Pear server\0.8"
   ```

2. **Build in release mode**
   ```powershell
   cargo build --release
   ```
   
   This will:
   - Download all dependencies
   - Compile with full optimizations
   - Create the binary at: `target\release\pear-server.exe`
   
   **Note**: First build may take 5-10 minutes as it downloads and compiles all dependencies.

## Step 3: Run Pear Server

```powershell
# Run directly with cargo
cargo run --release

# Or run the binary
.\target\release\pear-server.exe
```

## Step 4: Test the Server

Open a new PowerShell window and test:

```powershell
# Test HTTP/2 health endpoint  
curl http://localhost:8080/health

# Test stats endpoint
curl http://localhost:8080/stats

# Or use a web browser
# Navigate to: http://localhost:8080
```

## Step 5: Stop the Server

Press `Ctrl+C` in the terminal running Pear Server.

## Important Notes for Windows

### Performance Limitations
- Some Linux-specific optimizations (like `SO_REUSEPORT`) are not available on Windows
- The server will still run but may have slightly reduced performance compared to Linux
- For production deployment, Linux or WSL2 is strongly recommended

### Using WSL2 (Recommended)

For better performance and full Linux compatibility:

1. **Install WSL2**
   ```powershell
   wsl --install
   ```

2. **Install Ubuntu in WSL2**
   ```powershell
   wsl --install -d Ubuntu
   ```

3. **Inside WSL2, install Rust**
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

4. **Build and run in WSL2**
   ```bash
   cd "/mnt/c/Users/Elian/Desktop/Escritorio/Pear server/0.8"
   cargo build --release
   cargo run --release
   ```

### Firewall Configuration

Windows Firewall may prompt you to allow network access. Click "Allow" when prompted.

## Troubleshooting

### Issue: `cargo` not found
- **Solution**: Restart your terminal after installing Rust
- If still not working, manually add to PATH:
  - `C:\Users\<YourUser>\.cargo\bin`

### Issue: Build errors
- **Solution**: Ensure you have the latest Rust version:
  ```powershell
  rustup update
  ```

### Issue: Port already in use
- **Solution**: Change the ports in `src/network/config.rs`:
  ```rust
  http2_port: 8081,  // instead of 8080
  http3_port: 8444,  // instead of 8443
  ```

### Issue: Permission denied on port 80/443
- **Solution**: Use the default development ports (8080/8443)
- Windows doesn't have the same port privilege restrictions as Linux

## Next Steps

Once the server is running:
1. Check the logs in the terminal to see structured JSON output
2. Test the HTTP/2 endpoints with curl or a browser
3. Review the README.md for detailed architecture information
4. Wait for Phase 2 implementation for WebAssembly and Cage Pool features

## Getting Help

If you encounter issues:
1. Check the Rust installation: https://www.rust-lang.org/tools/install
2. Review error messages carefully
3. Ensure you have internet access for dependency downloads
4. Make sure you have at least 2GB free disk space for compilation

---

**Ready to build the future of web servers! 🍐🦀**
