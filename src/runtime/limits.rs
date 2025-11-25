// Linux resource limit management
// Handles file descriptor limits, memory limits, and system configuration

use anyhow::{Context, Result};
use tracing::{info, warn};

#[cfg(unix)]
use libc::{getrlimit, setrlimit, RLIMIT_NOFILE, rlimit};

/// Target file descriptor limit for high concurrency (1M connections)
const TARGET_FD_LIMIT: u64 = 1_048_576;

/// Minimum acceptable file descriptor limit
const MIN_FD_LIMIT: u64 = 65_536;

/// Set file descriptor limit to support millions of concurrent connections
#[cfg(unix)]
pub fn set_file_descriptor_limit() -> Result<()> {
    unsafe {
        let mut limit = rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };

        // Get current limits
        if getrlimit(RLIMIT_NOFILE, &mut limit) != 0 {
            return Err(anyhow::anyhow!("Failed to get file descriptor limit"));
        }

        info!(
            current_soft = limit.rlim_cur,
            current_hard = limit.rlim_max,
            "Current file descriptor limits"
        );

        // Try to set to target, but respect hard limit
        let new_limit = std::cmp::min(TARGET_FD_LIMIT, limit.rlim_max);
        
        if limit.rlim_cur < new_limit {
            limit.rlim_cur = new_limit;
            
            if setrlimit(RLIMIT_NOFILE, &limit) == 0 {
                info!(
                    new_limit = new_limit,
                    "Successfully increased file descriptor limit"
                );
            } else {
                warn!(
                    requested = new_limit,
                    "Failed to increase file descriptor limit - may need elevated privileges"
                );
            }
        }

        // Verify final limit
        if getrlimit(RLIMIT_NOFILE, &mut limit) == 0 {
            if limit.rlim_cur < MIN_FD_LIMIT {
                warn!(
                    current = limit.rlim_cur,
                    minimum = MIN_FD_LIMIT,
                    "File descriptor limit is below recommended minimum - connection capacity may be limited"
                );
            }
        }
    }

    Ok(())
}

#[cfg(not(unix))]
pub fn set_file_descriptor_limit() -> Result<()> {
    warn!("File descriptor limit configuration only supported on Unix systems");
    Ok(())
}

/// Log system information for diagnostics
pub fn log_system_info() {
    let num_cpus = num_cpus::get();
    let num_physical = num_cpus::get_physical();
    
    info!(
        logical_cpus = num_cpus,
        physical_cpus = num_physical,
        "System CPU information"
    );

    #[cfg(unix)]
    unsafe {
        let mut limit = rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        
        if getrlimit(RLIMIT_NOFILE, &mut limit) == 0 {
            info!(
                soft_limit = limit.rlim_cur,
                hard_limit = limit.rlim_max,
                "Final file descriptor limits"
            );
        }
    }
}
