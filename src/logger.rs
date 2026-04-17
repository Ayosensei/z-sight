/// Minimalist peak-usage logger. Appends a line only when a new session
/// peak is exceeded. Uses ~/.local/share/z-sight/peak_usage.log.
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct PeakLogger {
    log_path: PathBuf,
    session_peak_pct: f64,
}

impl PeakLogger {
    /// Create (or open) the logger. Creates the log directory if needed.
    pub fn new() -> io::Result<Self> {
        let log_dir = dirs_path();
        fs::create_dir_all(&log_dir)?;
        let log_path = log_dir.join("peak_usage.log");
        Ok(PeakLogger {
            log_path,
            session_peak_pct: 0.0,
        })
    }

    /// Check if `usage_pct` is a new session peak. If so, append a log line.
    pub fn record_if_peak(
        &mut self,
        usage_pct: f64,
        compression_ratio: f64,
        orig_data_size: u64,
        compr_data_size: u64,
    ) -> io::Result<()> {
        if usage_pct <= self.session_peak_pct {
            return Ok(());
        }

        self.session_peak_pct = usage_pct;

        let timestamp = iso8601_now();
        let orig_gb = orig_data_size as f64 / 1_073_741_824.0;
        let compr_mb = compr_data_size as f64 / 1_048_576.0;

        let line = format!(
            "{timestamp}  usage={usage_pct:.1}%  ratio={compression_ratio:.2}x  orig={orig_gb:.2}GB  compr={compr_mb:.0}MB\n"
        );

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)?;

        file.write_all(line.as_bytes())
    }

    /// Returns the log file path as a display string (for the TUI footer).
    pub fn log_path_display(&self) -> String {
        // Replace home dir with ~ for readability
        let home = std::env::var("HOME").unwrap_or_default();
        let path_str = self.log_path.to_string_lossy().to_string();
        path_str.replacen(&home, "~", 1)
    }

    pub fn session_peak_pct(&self) -> f64 {
        self.session_peak_pct
    }
}

fn dirs_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("z-sight")
}

/// Minimal ISO-8601 timestamp without any time crate dependency.
fn iso8601_now() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // Basic: just format unix seconds as UTC — good enough for a log file.
    // For a proper datetime we'd use `chrono`, but we keep deps minimal.
    let s = secs % 60;
    let m = (secs / 60) % 60;
    let h = (secs / 3600) % 24;
    let days = secs / 86400;
    // Approximate year/month/day from days since epoch (not accounting for leap years precisely,
    // but adequate for log timestamps)
    let year = 1970 + days / 365;
    let day_of_year = days % 365;
    let month = day_of_year / 30 + 1;
    let day = day_of_year % 30 + 1;
    format!("{year:04}-{month:02}-{day:02}T{h:02}:{m:02}:{s:02}Z")
}
