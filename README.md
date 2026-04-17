# ⚡ Z-Sight

> Lightweight ZRAM monitor TUI for Linux — built for 4GB RAM laptops running Zorin OS

![Rust](https://img.shields.io/badge/rust-1.75%2B-orange?style=flat-square)
![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)

## What It Does

Z-Sight is a real-time terminal dashboard that monitors ZRAM performance on your Linux system. It gives you live insight into:

- **ZRAM usage %** — how much of your virtual swap is consumed
- **Compression ratio** — how efficiently ZRAM is compressing your memory
- **System RAM** — total / used / available at a glance
- **2-minute sparkline history** — for both usage and compression ratio
- **Desktop alerts** — native notifications when thresholds are breached
- **Peak usage logging** — automatic append-only log of session highs

## Screenshot

```
╔══════════════════════════════════════════════════════════════════╗
║  ⚡ Z-SIGHT  ·  ZRAM Monitor              Zorin OS  ·  4.0 GB  ║
╠══════════════════════╦═══════════════════════════════════════════╣
║  ZRAM DEVICE: zram0  ║  SYSTEM MEMORY                          ║
║                      ║  RAM Used  [████████░░]  72.3%          ║
║  [████████░░] 72.4%  ║  Total 4.0 GB    Available 1.1 GB       ║
║  Disksize   4.0 GB   ╠═════════════════╦═════════════════════════╣
║  Orig Data  2.1 GB   ║ RATIO HISTORY   ║ USAGE HISTORY          ║
║  Compr Data 931 MB   ║  · · · 1.5x ·  ║  · · · 85% limit · ·  ║
║  Overhead   1.04 GB  ║  ▃▅▆▇▆▇▇▆▇▇▇▆  ║  ▂▃▄▄▅▅▆▆▆▇▇▇▇▇▇▇▇     ║
║  Ratio 2.31x  ●      ╚═════════════════╩═════════════════════════╣
║  Health  NORMAL      ║  ALERT STATUS                           ║
╠══════════════════════╣  Usage > 85%       ✅ OK                ║
║  Last alert:  --     ║  Ratio < 1.5x      ✅ OK                ║
╠══════════════════════╩═══════════════════════════════════════════╣
║  [q] Quit   [p] Pause     Peak: 72.4%  ·  Log: ~/.local/...   ║
╚══════════════════════════════════════════════════════════════════╝
```

## Requirements

- Linux with ZRAM enabled (`/sys/block/zram0/` must exist)
- Rust 1.75+
- `libnotify` (for desktop alerts — already installed on Zorin OS)

## Build & Run

```bash
git clone <repo>
cd z-sight

# Dev build
cargo run

# Optimised release build (~small binary, stripped)
cargo build --release
./target/release/z-sight
```

## Keybindings

| Key | Action |
|-----|--------|
| `q` | Quit |
| `p` | Pause / Resume live updates |

## Alert Thresholds

| Metric | Threshold | Meaning |
|--------|-----------|---------|
| ZRAM usage | > 85% | Swap pressure is high |
| Compression ratio | < 1.5x | Storing mostly uncompressible data |

Alerts observe a **60-second cooldown** to prevent notification spam.

## Logging

Session peak usage is appended to:
```
~/.local/share/z-sight/peak_usage.log
```

Example entry:
```
2026-04-17T22:00:00Z  usage=91.2%  ratio=1.3x  orig=2.10GB  compr=923MB
```

Use this log over time to tune `vm.swappiness` for your workload.

## Safety

- **Read-only**: only reads from `/sys/block/zram0/` and `/proc`. Never writes to system parameters.
- **Low footprint**: targets < 4 MB RSS while running.
- **2-second poll interval**: minimal CPU wakeup impact.

## Architecture

```
src/
├── main.rs      Event loop, terminal init/restore, history buffers
├── zram.rs      /sys/block/zram0/ reader, stat derivation, unit tests
├── system.rs    System RAM context via sysinfo
├── alerts.rs    Desktop notification dispatch with cooldown logic
├── logger.rs    Append-only peak usage logger
└── ui.rs        Multi-panel ratatui TUI (gauges, sparklines, alerts)
```
