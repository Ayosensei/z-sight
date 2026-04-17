mod alerts;
mod logger;
mod processes;
mod swap;
mod system;
mod ui;
mod zram;

use std::collections::VecDeque;
use std::io;
use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use sysinfo::System;

/// Rolling history length: 60 ticks × 2s = 2 minutes.
const HISTORY_LEN: usize = 60;

/// Tick interval in milliseconds.
const TICK_MS: u64 = 2000;

fn main() -> io::Result<()> {
    // ── Setup terminal ───────────────────────────────────────────────────────
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // ── State ────────────────────────────────────────────────────────────────
    let mut sys = System::new_all();
    let mut alert_state = alerts::AlertState::new();
    let mut peak_logger = logger::PeakLogger::new()?;
    let mut usage_history: VecDeque<f64> = VecDeque::with_capacity(HISTORY_LEN);
    let mut ratio_history: VecDeque<f64> = VecDeque::with_capacity(HISTORY_LEN);
    let mut swap_entries = swap::read_entries().unwrap_or_default();
    let mut swap_totals = swap::totals(&swap_entries);
    let mut proc_scanner = processes::ProcessScanner::new();
    let mut paused = false;
    let mut tick_count: u64 = 0;

    // ── Initial read (so we have data on first frame) ────────────────────────
    let mut zram_stats = zram::read_stats().unwrap_or_else(|_| fallback_zram());
    let mut ram_stats = system::read_stats(&mut sys);
    let mut top_processes = proc_scanner.top(ram_stats.total);
    push_history(&mut usage_history, zram_stats.usage_pct);
    push_history(&mut ratio_history, zram_stats.compression_ratio);

    // ── Event loop ───────────────────────────────────────────────────────────
    loop {
        // Draw current frame
        {
            let log_path = peak_logger.log_path_display();
            terminal.draw(|f| {
                ui::draw(
                    f,
                    &ui::AppState {
                        zram: &zram_stats,
                        ram: &ram_stats,
                        alerts: &alert_state,
                        swap_entries: &swap_entries,
                        swap_totals: &swap_totals,
                        top_processes: &top_processes,
                        usage_history: &usage_history,
                        ratio_history: &ratio_history,
                        session_peak_pct: peak_logger.session_peak_pct(),
                        log_path: &log_path,
                        paused,
                        tick_count,
                    },
                );
            })?;
        }

        // Poll for keyboard events with a 2-second timeout
        if event::poll(Duration::from_millis(TICK_MS))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') => break,
                        KeyCode::Char('p') | KeyCode::Char('P') => {
                            paused = !paused;
                        }
                        _ => {}
                    }
                }
            }
        }

        // Skip update if paused
        if paused {
            continue;
        }

        tick_count += 1;

        // ── Refresh data ─────────────────────────────────────────────────────
        if let Ok(stats) = zram::read_stats() {
            zram_stats = stats;
        }
        ram_stats = system::read_stats(&mut sys);
        if let Ok(entries) = swap::read_entries() {
            swap_totals = swap::totals(&entries);
            swap_entries = entries;
        }
        top_processes = proc_scanner.top(ram_stats.total);

        // ── Update histories ─────────────────────────────────────────────────
        push_history(&mut usage_history, zram_stats.usage_pct);
        push_history(&mut ratio_history, zram_stats.compression_ratio);

        // ── Alerts ───────────────────────────────────────────────────────────
        alert_state.check(zram_stats.usage_pct, zram_stats.compression_ratio);

        // ── Logging ──────────────────────────────────────────────────────────
        let _ = peak_logger.record_if_peak(
            zram_stats.usage_pct,
            zram_stats.compression_ratio,
            zram_stats.orig_data_size,
            zram_stats.compr_data_size,
        );
    }

    // ── Restore terminal ─────────────────────────────────────────────────────
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

/// Push a value into a capped VecDeque history buffer.
fn push_history(buf: &mut VecDeque<f64>, value: f64) {
    if buf.len() == HISTORY_LEN {
        buf.pop_front();
    }
    buf.push_back(value);
}

/// Returns a zeroed ZramStats for use when sysfs is unavailable (e.g. non-Linux dev machine).
fn fallback_zram() -> zram::ZramStats {
    zram::ZramStats {
        disksize: 0,
        orig_data_size: 0,
        compr_data_size: 0,
        mem_used_total: 0,
        usage_pct: 0.0,
        compression_ratio: 0.0,
        health: zram::Health::Normal,
    }
}
