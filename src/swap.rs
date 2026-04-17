/// Reads all active swap entries from /proc/swaps.
use std::fs;
use std::io;

/// A single active swap device/file.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SwapEntry {
    /// Full path, e.g. "/swapfile" or "/dev/zram0"
    pub name: String,
    /// Short display name (basename)
    pub display_name: String,
    /// "file" or "partition"
    pub kind: String,
    /// Total size in bytes
    pub size: u64,
    /// Currently used bytes
    pub used: u64,
    /// Usage percentage (0.0–100.0)
    pub used_pct: f64,
    /// Kernel priority (higher = preferred)
    pub priority: i32,
}

/// Summary totals across all swap entries.
#[derive(Debug, Clone, Default)]
pub struct SwapTotals {
    pub total_size: u64,
    pub total_used: u64,
    pub used_pct: f64,
}

/// Parse /proc/swaps and return all active swap entries.
/// Format (tab/space separated, sizes in KiB):
///   Filename  Type  Size  Used  Priority
pub fn read_entries() -> io::Result<Vec<SwapEntry>> {
    let raw = fs::read_to_string("/proc/swaps")?;
    let mut entries = Vec::new();

    for line in raw.lines().skip(1) {
        // Split on any whitespace
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 5 {
            continue;
        }

        let name = cols[0].to_string();
        let display_name = name
            .rsplit('/')
            .next()
            .unwrap_or(&name)
            .to_string();

        let kind = cols[1].to_string();

        // /proc/swaps reports sizes in KiB
        let size_kib: u64 = cols[2].parse().unwrap_or(0);
        let used_kib: u64 = cols[3].parse().unwrap_or(0);
        let priority: i32 = cols[4].parse().unwrap_or(0);

        let size = size_kib * 1024;
        let used = used_kib * 1024;
        let used_pct = if size == 0 {
            0.0
        } else {
            (used as f64 / size as f64) * 100.0
        };

        entries.push(SwapEntry {
            name,
            display_name,
            kind,
            size,
            used,
            used_pct,
            priority,
        });
    }

    // Sort by priority descending (highest priority first)
    entries.sort_by(|a, b| b.priority.cmp(&a.priority));
    Ok(entries)
}

/// Compute totals across all entries.
pub fn totals(entries: &[SwapEntry]) -> SwapTotals {
    let total_size: u64 = entries.iter().map(|e| e.size).sum();
    let total_used: u64 = entries.iter().map(|e| e.used).sum();
    let used_pct = if total_size == 0 {
        0.0
    } else {
        (total_used as f64 / total_size as f64) * 100.0
    };
    SwapTotals {
        total_size,
        total_used,
        used_pct,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(size: u64, used: u64) -> SwapEntry {
        SwapEntry {
            name: "/test".into(),
            display_name: "test".into(),
            kind: "file".into(),
            size,
            used,
            used_pct: if size == 0 { 0.0 } else { used as f64 / size as f64 * 100.0 },
            priority: 0,
        }
    }

    #[test]
    fn totals_basic() {
        let entries = vec![
            make_entry(4 * 1024 * 1024 * 1024, 1024 * 1024 * 1024),
            make_entry(2 * 1024 * 1024 * 1024, 1024 * 1024 * 1024),
        ];
        let t = totals(&entries);
        assert_eq!(t.total_size, 6 * 1024 * 1024 * 1024);
        assert_eq!(t.total_used, 2 * 1024 * 1024 * 1024);
        assert!((t.used_pct - 33.33).abs() < 0.1);
    }

    #[test]
    fn totals_empty() {
        let t = totals(&[]);
        assert_eq!(t.used_pct, 0.0);
    }
}
