/// System-wide RAM statistics via the `sysinfo` crate.
use sysinfo::System;

/// General system memory statistics (all values in bytes).
#[derive(Debug, Clone)]
pub struct RamStats {
    pub total: u64,
    pub used: u64,
    pub available: u64,
    /// Percentage of system RAM currently in use (0.0–100.0)
    pub used_pct: f64,
}

/// Refresh system info and return a RamStats snapshot.
pub fn read_stats(sys: &mut System) -> RamStats {
    sys.refresh_memory();

    let total = sys.total_memory();
    let used = sys.used_memory();
    let available = sys.available_memory();
    let used_pct = if total == 0 {
        0.0
    } else {
        (used as f64 / total as f64) * 100.0
    };

    RamStats {
        total,
        used,
        available,
        used_pct,
    }
}
