# Z-Sight User Manual

> **Z-Sight** — ZRAM Monitor TUI for Linux  
> Version 0.1.0 · Built for Zorin OS / Ubuntu-based systems

---

## Table of Contents

1. [What is ZRAM?](#1-what-is-zram)
2. [Installation](#2-installation)
3. [Launching Z-Sight](#3-launching-z-sight)
4. [Reading the Dashboard](#4-reading-the-dashboard)
   - [Header Bar](#41-header-bar)
   - [ZRAM Device Panel](#42-zram-device-panel)
   - [System Memory Panel](#43-system-memory-panel)
   - [Sparkline History Charts](#44-sparkline-history-charts)
   - [Alert Status Panel](#45-alert-status-panel)
   - [Footer Bar](#46-footer-bar)
5. [Keybindings](#5-keybindings)
6. [Understanding the Metrics](#6-understanding-the-metrics)
7. [The Alert System](#7-the-alert-system)
8. [The Peak Usage Log](#8-the-peak-usage-log)
9. [Health Classifications](#9-health-classifications)
10. [Troubleshooting](#10-troubleshooting)

---

## 1. What is ZRAM?

ZRAM is a Linux kernel feature that creates a **compressed RAM disk** used as swap space. Instead of writing to your slow SSD when RAM fills up, the kernel compresses less-used memory pages and stores them back in RAM — much faster than disk-based swap.

On a 4 GB laptop like yours, Zorin OS typically configures a 4 GB ZRAM device (`zram0`). This effectively gives you more usable memory headroom, at the cost of CPU cycles for compression.

**Why monitor it?**  
ZRAM is silent by default. Without a tool like Z-Sight, you have no visibility into whether your system is under memory pressure, how efficiently ZRAM is compressing your data, or when you're approaching the limits of your swap headroom.

---

## 2. Installation

Z-Sight is installed as a native Rust binary via Cargo:

```bash
cargo install --path "/path/to/z-sight"
```

After installation, the binary lives at `~/.cargo/bin/z-sight` and is available system-wide as the `z-sight` command.

**To update** after pulling new changes from the repo, re-run the same install command — Cargo will replace the old binary automatically.

**To uninstall:**
```bash
cargo uninstall z-sight
```

---

## 3. Launching Z-Sight

Open any terminal and run:

```bash
z-sight
```

The TUI takes over the full terminal window immediately. Your previous terminal content is preserved and restored when you quit.

> [!NOTE]
> Z-Sight works best in a terminal window at least **80 columns × 24 rows**. Make it larger for more comfortable sparkline charts.

---

## 4. Reading the Dashboard

The dashboard is divided into panels. Here is a breakdown of each one.

---

### 4.1 Header Bar

```
║  ⚡ Z-SIGHT  ·  ZRAM Monitor              Zorin OS  ·  4.0 GB RAM  ║
```

| Element | Meaning |
|---|---|
| `⚡ Z-SIGHT` | App name |
| `ZRAM Monitor` | Mode identifier |
| `⏸ PAUSED` | Appears here when updates are paused with `p` |
| `Zorin OS · 4.0 GB RAM` | Your OS and total physical RAM |

---

### 4.2 ZRAM Device Panel

The main panel on the **left side** of the screen. Shows real-time stats for `zram0`.

```
║  ZRAM DEVICE: zram0      ║
║                          ║
║  Usage                   ║
║  [████████████░░░░] 72%  ║
║                          ║
║  Disksize    4.0 GB      ║
║  Orig Data   2.1 GB      ║
║  Compr Data  931 MB      ║
║  Overhead    1.04 GB     ║
║                          ║
║  Ratio       2.31x  ●    ║
║  Health      NORMAL      ║
```

| Field | What it Means |
|---|---|
| **Usage gauge** | Percentage of ZRAM disksize consumed (`Overhead ÷ Disksize`). Color shifts green → yellow → red. |
| **Disksize** | The total virtual swap space your ZRAM device is configured to hold. |
| **Orig Data** | The raw, uncompressed size of data currently stored in ZRAM. |
| **Compr Data** | How much physical RAM that data actually occupies after compression. |
| **Overhead** | Total physical RAM used by ZRAM including metadata bookkeeping. Slightly larger than Compr Data. |
| **Ratio** | `Orig Data ÷ Compr Data`. Higher is better — means ZRAM is efficiently compressing your memory. |
| **Health** | A summary label derived from both Usage and Ratio. See [Section 9](#9-health-classifications). |

**Gauge colours:**

| Colour | Usage Range |
|---|---|
| 🟢 Green | Below 70% |
| 🟡 Yellow | 70% – 85% |
| 🔴 Red | Above 85% |

---

### 4.3 System Memory Panel

The panel on the **top right**. Shows your overall physical RAM state, independent of ZRAM.

```
║  SYSTEM MEMORY                           ║
║  RAM Used  [████████░░]  72.3%           ║
║  Total 4.0 GB    Available 1.1 GB        ║
```

| Field | What it Means |
|---|---|
| **RAM Used gauge** | Percentage of system RAM in active use. |
| **Total** | Your physical RAM capacity. |
| **Available** | RAM immediately available to new processes (includes reclaimable cache). |

> [!TIP]
> Watch this panel alongside the ZRAM panel. If System RAM is near 100% *and* ZRAM usage is climbing, your system is under serious memory pressure and you should consider closing applications.

---

### 4.4 Sparkline History Charts

The **bottom right** area shows two rolling charts, each covering the **last 2 minutes** of data (one data point every 2 seconds, 60 points total).

#### Compression Ratio History
```
║  RATIO HISTORY  2.31x                    ║
║  · · · · · · · · · · · 1.5x limit · ·   ║
║  ▃▅▆▇▆▇▇▇▆▇▆▇▅▇▇▆▇▆▇▇▆▇▇▆▇▇▇▆▇▆▇▇▆      ║
```

- The **title shows the current live value**.
- The dashed line marks the **1.5x alert threshold**.
- Bars growing taller = better compression.
- A sudden drop toward the threshold line means your workload has shifted to uncompressible data (e.g. video files, encrypted content).

#### Usage History
```
║  USAGE HISTORY  72.4%                    ║
║  · · · · · · · · · · · 85% limit · ·    ║
║  ▂▃▄▄▅▅▆▆▆▇▇▇▇▇▇▇▇▇                     ║
```

- The **title shows the current live value**.
- The dashed line marks the **85% alert threshold**.
- Steadily growing bars = memory pressure building up over time.

---

### 4.5 Alert Status Panel

The panel on the **bottom left**, below the ZRAM stats.

```
║  ALERT STATUS                 ║
║  Usage > 85%       ✅ OK      ║
║  Ratio < 1.5x      ✅ OK      ║
║  Last alert:  --              ║
```

| State | Meaning |
|---|---|
| `✅ OK` (green) | Threshold is not breached |
| `⚠️ ALERT` (amber) | Threshold breaching, notification recently sent |
| `🔴 ALERT` (red) | Actively breached |
| **Last alert** | How long ago the most recent desktop notification was dispatched. `--` means no alert has fired this session. |

---

### 4.6 Footer Bar

```
║  [q] Quit   [p] Pause     Peak: 72.4%  ·  Log: ~/.local/share/z-sight/...  ║
```

| Element | Meaning |
|---|---|
| `[q] Quit` | Key to exit the tool cleanly |
| `[p] Pause / Resume` | Key to freeze/unfreeze live updates |
| **Peak** | The highest ZRAM usage % recorded this session |
| **Log** | Path to your peak usage log file |

---

## 5. Keybindings

| Key | Action |
|---|---|
| `q` | Quit Z-Sight and return to your normal terminal |
| `p` | **Pause** live updates (the display freezes, no new reads) |
| `p` (again) | **Resume** live updates |

> [!NOTE]
> Pausing does **not** stop alerts or logging — those are suspended along with the tick loop. Pausing is purely for reading the current values without the display refreshing.

---

## 6. Understanding the Metrics

### Compression Ratio — The Most Important Number

The compression ratio tells you how hard ZRAM is working and how effectively it's giving you extra memory headroom.

| Ratio | What's Happening |
|---|---|
| **3.0x – 5.0x** | Excellent. Mostly text/code in memory. Efficient zone. |
| **2.0x – 3.0x** | Good. Healthy mixed workload. |
| **1.5x – 2.0x** | Adequate but watch the trend. |
| **Below 1.5x** | ⚠️ Warning. Mostly uncompressible data in memory (browser media, temp files, etc). ZRAM is providing little benefit and consuming more RAM than it saves. |
| **Below 1.0x** | Rare edge case — would mean compressed data is *larger* than the original. |

### ZRAM Usage % — Your Swap Headroom

This tells you how much of your virtual swap budget is consumed.

- **Below 70%** — Comfortable. ZRAM has plenty of room.
- **70–85%** — Watch it. Memory pressure is elevated. Consider closing unused apps.
- **Above 85%** — Alert threshold. If ZRAM fills up completely and system RAM is exhausted, the OOM (Out Of Memory) killer will start terminating processes.

---

## 7. The Alert System

Z-Sight sends **desktop notifications** via your system's notification daemon (libnotify on Zorin OS) when either threshold is exceeded:

| Alert | Threshold | Message |
|---|---|---|
| ZRAM Pressure | Usage > **85%** | "ZRAM usage is X% (threshold: 85%)" |
| Low Compression | Ratio < **1.5x** | "Compression ratio is Xx — uncompressible memory load detected." |

### Cooldown

Each alert type has an independent **60-second cooldown**. Once an alert fires, it will not fire again for the same condition until 60 seconds have passed. This prevents notification spam during sustained pressure.

### What to Do When You Get an Alert

**High Usage alert:**
1. Check which apps are using the most memory: `ps aux --sort=-%mem | head -10`
2. Close browser tabs, media players, or other heavy applications
3. Watch ZRAM usage drop on the sparkline

**Low Compression alert:**
1. This usually means you have a lot of media (video, audio, images) or encrypted data in memory
2. Closing media applications will typically restore a healthy ratio
3. If it persists, your workload may simply not be a good fit for ZRAM compression

---

## 8. The Peak Usage Log

Z-Sight automatically appends a line to your log file **every time a new session peak is reached**:

```
~/.local/share/z-sight/peak_usage.log
```

**Example entries:**
```
2026-04-17T22:00:00Z  usage=72.4%  ratio=2.31x  orig=2.10GB  compr=923MB
2026-04-17T22:15:00Z  usage=81.2%  ratio=1.87x  orig=2.80GB  compr=1490MB
2026-04-17T22:48:00Z  usage=91.8%  ratio=1.31x  orig=3.10GB  compr=2360MB
```

**Useful commands:**

```bash
# View the full log
cat ~/.local/share/z-sight/peak_usage.log

# Watch for new entries live
tail -f ~/.local/share/z-sight/peak_usage.log

# Count how many peaks recorded
wc -l ~/.local/share/z-sight/peak_usage.log
```

### Using the Log to Tune Your System

Over time, the log reveals your memory usage patterns. If you consistently see peaks above 80%, consider:

- Reducing `vm.swappiness` to make Linux less eager to push data to swap:
  ```bash
  # Check current value (default is usually 60 or 100)
  cat /proc/sys/vm/swappiness

  # Lower it (e.g. to 30) — persists until reboot
  sudo sysctl vm.swappiness=30
  ```
- Closing more applications during heavy workloads
- Considering a RAM upgrade if consistently hitting 85%+

---

## 9. Health Classifications

Z-Sight derives an overall **Health** label from the combination of ZRAM usage and compression ratio:

| Label | Colour | Conditions |
|---|---|---|
| `NORMAL` | 🟢 Green | Usage ≤ 70% **and** Ratio ≥ 2.0x |
| `PRESSURE` | 🟡 Yellow | Usage 70–85% **or** Ratio 1.5–2.0x |
| `CRITICAL` | 🔴 Red | Usage > 85% **or** Ratio < 1.5x |

The health label is the quickest at-a-glance status — if it's green, you're fine.

---

## 10. Troubleshooting

### Z-Sight shows all zeros

Your ZRAM device may not have any data in it yet. This is normal if the system was just booted and memory pressure hasn't pushed anything into swap yet. The values will populate as the system runs.

### "zram0 not found" / tool crashes on startup

ZRAM may not be enabled on your system. Check:
```bash
ls /sys/block/ | grep zram
```

If nothing appears, ZRAM is not active. On Zorin OS it should be enabled by default — if not, check your system settings or reinstall the `zram-config` package:
```bash
sudo apt install zram-config
```

### Desktop notifications aren't appearing

Ensure a notification daemon is running:
```bash
notify-send "test" "Z-Sight alert test"
```

If the test notification doesn't appear, check your Zorin OS notification settings or ensure `libnotify-bin` is installed:
```bash
sudo apt install libnotify-bin
```

### The TUI looks broken / garbled

Your terminal may not support the Unicode block characters used by the sparklines. Try switching to a terminal that supports UTF-8 (e.g. GNOME Terminal, Alacritty, Kitty). Also ensure your terminal font includes the Unicode block character range.

### I want to run Z-Sight automatically at login

Add it to your desktop autostart, or for a terminal-based approach, add to your shell profile:
```bash
# Append to ~/.bashrc or ~/.zshrc
alias zs='z-sight'
```

For a persistent background monitor, consider running it in a `tmux` session.
