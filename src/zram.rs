/// Reads ZRAM statistics from /sys/block/zram0/ and derives key metrics.
use std::io;
use std::fs;

const ZRAM_PATH: &str = "/sys/block/zram0";

/// Raw + derived ZRAM statistics.
#[derive(Debug, Clone)]
pub struct ZramStats {
    /// Total virtual swap space configured (bytes)
    pub disksize: u64,
    /// Uncompressed data currently stored in ZRAM (bytes)
    pub orig_data_size: u64,
    /// Physical RAM occupied by compressed data (bytes)
    pub compr_data_size: u64,
    /// Total RAM overhead including metadata (bytes)
    pub mem_used_total: u64,
    /// Percentage of disksize consumed by mem_used_total (0.0–100.0)
    pub usage_pct: f64,
    /// Compression efficiency: orig_data_size / compr_data_size
    pub compression_ratio: f64,
    /// Overall health classification
    pub health: Health,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Health {
    Normal,
    Pressure,
    Critical,
}

impl Health {
    pub fn label(&self) -> &'static str {
        match self {
            Health::Normal => "NORMAL",
            Health::Pressure => "PRESSURE",
            Health::Critical => "CRITICAL",
        }
    }
}

/// Read and parse a single u64 from a sysfs file.
pub fn read_u64(filename: &str) -> io::Result<u64> {
    let path = format!("{ZRAM_PATH}/{filename}");
    let raw = fs::read_to_string(&path)?;
    raw.trim()
        .parse::<u64>()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// Parsed fields from /sys/block/zram0/mm_stat.
/// Format: orig_data_size compr_data_size mem_used_total mem_limit mem_used_max ...
pub struct MmStat {
    pub orig_data_size: u64,
    pub compr_data_size: u64,
    pub mem_used_total: u64,
}

/// Parse the mm_stat line (modern kernels consolidate stats here).
pub fn parse_mm_stat(line: &str) -> io::Result<MmStat> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    if fields.len() < 3 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "mm_stat has fewer than 3 fields",
        ));
    }
    let parse = |s: &str| {
        s.parse::<u64>()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    };
    Ok(MmStat {
        orig_data_size: parse(fields[0])?,
        compr_data_size: parse(fields[1])?,
        mem_used_total: parse(fields[2])?,
    })
}

/// Read mm_stat from sysfs.
fn read_mm_stat() -> io::Result<MmStat> {
    let path = format!("{ZRAM_PATH}/mm_stat");
    let raw = fs::read_to_string(&path)?;
    parse_mm_stat(raw.trim())
}

/// Compute usage percentage given `mem_used_total` and `disksize`.
pub fn calc_usage_pct(mem_used_total: u64, disksize: u64) -> f64 {
    if disksize == 0 {
        return 0.0;
    }
    (mem_used_total as f64 / disksize as f64) * 100.0
}

/// Compute compression ratio given `orig_data_size` and `compr_data_size`.
pub fn calc_compression_ratio(orig: u64, compr: u64) -> f64 {
    if compr == 0 {
        return 0.0;
    }
    orig as f64 / compr as f64
}

/// Classify health from usage % and compression ratio.
pub fn classify_health(usage_pct: f64, compression_ratio: f64) -> Health {
    if usage_pct > 85.0 || compression_ratio < 1.5 {
        Health::Critical
    } else if usage_pct > 70.0 || compression_ratio < 2.0 {
        Health::Pressure
    } else {
        Health::Normal
    }
}

/// Read all ZRAM stats from sysfs and return a populated ZramStats struct.
/// Uses mm_stat (modern kernels) for per-field stats.
pub fn read_stats() -> io::Result<ZramStats> {
    let disksize = read_u64("disksize")?;
    let mm = read_mm_stat()?;

    let usage_pct = calc_usage_pct(mm.mem_used_total, disksize);
    let compression_ratio = calc_compression_ratio(mm.orig_data_size, mm.compr_data_size);
    let health = classify_health(usage_pct, compression_ratio);

    Ok(ZramStats {
        disksize,
        orig_data_size: mm.orig_data_size,
        compr_data_size: mm.compr_data_size,
        mem_used_total: mm.mem_used_total,
        usage_pct,
        compression_ratio,
        health,
    })
}

// ── Unit Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_pct_normal() {
        // 1 GB used out of 4 GB = 25%
        let pct = calc_usage_pct(1_073_741_824, 4_294_967_296);
        assert!((pct - 25.0).abs() < 0.01);
    }

    #[test]
    fn usage_pct_zero_disksize() {
        assert_eq!(calc_usage_pct(100, 0), 0.0);
    }

    #[test]
    fn compression_ratio_normal() {
        // 2 GB orig, 1 GB compressed = 2.0x
        let ratio = calc_compression_ratio(2_147_483_648, 1_073_741_824);
        assert!((ratio - 2.0).abs() < 0.001);
    }

    #[test]
    fn compression_ratio_zero_compr() {
        assert_eq!(calc_compression_ratio(100, 0), 0.0);
    }

    #[test]
    fn health_normal() {
        assert_eq!(classify_health(50.0, 2.5), Health::Normal);
    }

    #[test]
    fn health_pressure_high_usage() {
        assert_eq!(classify_health(75.0, 2.5), Health::Pressure);
    }

    #[test]
    fn health_pressure_low_ratio() {
        assert_eq!(classify_health(50.0, 1.8), Health::Pressure);
    }

    #[test]
    fn health_critical_usage() {
        assert_eq!(classify_health(90.0, 2.5), Health::Critical);
    }

    #[test]
    fn health_critical_ratio() {
        assert_eq!(classify_health(50.0, 1.2), Health::Critical);
    }

    #[test]
    fn parse_mm_stat_valid() {
        // Sample line matching real kernel output
        let line = "1825492992 601143966 614608896 0 810680320 38614 1029435 26721 475378";
        let s = parse_mm_stat(line).unwrap();
        assert_eq!(s.orig_data_size, 1_825_492_992);
        assert_eq!(s.compr_data_size, 601_143_966);
        assert_eq!(s.mem_used_total, 614_608_896);
    }

    #[test]
    fn parse_mm_stat_too_short() {
        assert!(parse_mm_stat("100 200").is_err());
    }
}
