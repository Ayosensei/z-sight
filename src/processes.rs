/// Top-N processes by memory usage, sourced via sysinfo.
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};

/// Number of processes shown in the leaderboard.
pub const TOP_N: usize = 5;

/// Memory info for a single process.
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    /// Truncated display name
    pub name: String,
    /// Resident Set Size (RSS) in bytes
    pub memory: u64,
    /// RSS as % of total system RAM
    pub memory_pct: f64,
}

/// A dedicated lightweight System instance for process scanning.
/// Kept separate so we only refresh what we need.
pub struct ProcessScanner {
    sys: System,
}

impl ProcessScanner {
    pub fn new() -> Self {
        ProcessScanner {
            sys: System::new_with_specifics(
                RefreshKind::nothing().with_processes(
                    ProcessRefreshKind::nothing().with_memory(),
                ),
            ),
        }
    }

    /// Refresh and return the top `TOP_N` processes sorted by RSS descending.
    pub fn top(&mut self, total_ram: u64) -> Vec<ProcessInfo> {
        self.sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing().with_memory(),
        );

        let mut procs: Vec<_> = self.sys.processes().values()
            .filter(|p| p.thread_kind().is_none()) // exclude Linux threads
            .collect();
        procs.sort_by_key(|p| std::cmp::Reverse(p.memory()));

        procs
            .iter()
            .take(TOP_N)
            .map(|p| {
                let memory = p.memory();
                let memory_pct = if total_ram == 0 {
                    0.0
                } else {
                    memory as f64 / total_ram as f64 * 100.0
                };
                // Truncate long names so they fit the panel
                let raw = p.name().to_string_lossy();
                let name = if raw.len() > 14 {
                    format!("{}…", &raw[..13])
                } else {
                    raw.into_owned()
                };
                ProcessInfo {
                    name,
                    memory,
                    memory_pct,
                }
            })
            .collect()
    }
}
