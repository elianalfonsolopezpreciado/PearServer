// Bind Mount Configuration for Wasmtime
// Zero-copy shared access using preopened directories

use wasmtime::*;
use wasmtime_wasi::{WasiCtxBuilder, Dir, DirPerms, FilePerms};
use std::path::Path;
use anyhow::{Result, Context};
use tracing::debug;

/// Configure bind mount for a Cage
pub fn configure_bind_mount<P: AsRef<Path>>(
    wasi_builder: WasiCtxBuilder,
    host_path: P,
    guest_path: &str,
) -> Result<WasiCtxBuilder> {
    let host_path = host_path.as_ref();
    
    debug!(
        host_path = %host_path.display(),
        guest_path = %guest_path,
        "Configuring bind mount"
    );

    // Open host directory for reading
    let dir = Dir::open_ambient_dir(host_path, wasmtime_wasi::ambient_authority())
        .with_context(|| format!("Failed to open host directory: {}", host_path.display()))?;

    // Preopen directory with read-only permissions
    let wasi_builder = wasi_builder
        .preopened_dir(dir, DirPerms::READ, FilePerms::READ, guest_path)?;

    Ok(wasi_builder)
}

/// Configure read-write bind mount (for isolated modifications)
pub fn configure_rw_bind_mount<P: AsRef<Path>>(
    wasi_builder: WasiCtxBuilder,
    host_path: P,
    guest_path: &str,
) -> Result<WasiCtxBuilder> {
    let host_path = host_path.as_ref();
    
    debug!(
        host_path = %host_path.display(),
        guest_path = %guest_path,
        "Configuring read-write bind mount"
    );

    let dir = Dir::open_ambient_dir(host_path, wasmtime_wasi::ambient_authority())
        .with_context(|| format!("Failed to open host directory: {}", host_path.display()))?;

    // Preopen with read-write permissions
    let wasi_builder = wasi_builder
        .preopened_dir(
            dir,
            DirPerms::READ | DirPerms::MUTATE,
            FilePerms::READ | FilePerms::WRITE,
            guest_path
        )?;

    Ok(wasi_builder)
}

/// Standard bind mount configuration for web applications
pub fn standard_web_mounts<P: AsRef<Path>>(
    wasi_builder: WasiCtxBuilder,
    site_path: P,
) -> Result<WasiCtxBuilder> {
    let site_path = site_path.as_ref();
    
    // Mount site files at /var/www (read-only)
    let wasi_builder = configure_bind_mount(wasi_builder, site_path, "/var/www")?;
    
    // Create temp directory for uploads/cache (read-write)
    let temp_path = site_path.join("tmp");
    std::fs::create_dir_all(&temp_path)?;
    let wasi_builder = configure_rw_bind_mount(wasi_builder, temp_path, "/tmp")?;
    
    debug!("Standard web mounts configured");
    
    Ok(wasi_builder)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_bind_mount_configuration() {
        let temp = TempDir::new().unwrap();
        let wasi_builder = WasiCtxBuilder::new();
        
        let result = configure_bind_mount(wasi_builder, temp.path(), "/test");
        assert!(result.is_ok());
    }

    #[test]
    fn test_standard_web_mounts() {
        let temp = TempDir::new().unwrap();
        let wasi_builder = WasiCtxBuilder::new();
        
        let result = standard_web_mounts(wasi_builder, temp.path());
        assert!(result.is_ok());
    }
}
