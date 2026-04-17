/// Multi-panel ratatui TUI for Z-Sight.
use std::collections::VecDeque;

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Gauge, Paragraph, Sparkline, Wrap,
    },
};

use crate::alerts::{AlertState, RATIO_THRESHOLD, USAGE_THRESHOLD};
use crate::swap::{SwapEntry, SwapTotals};
use crate::system::RamStats;
use crate::zram::{Health, ZramStats};

// ── Colour palette ───────────────────────────────────────────────────────────

const C_BRAND: Color = Color::Rgb(100, 210, 255);   // Icy blue — brand accent
const C_OK: Color = Color::Rgb(80, 220, 130);        // Vibrant green
const C_WARN: Color = Color::Rgb(255, 200, 60);      // Amber
const C_CRIT: Color = Color::Rgb(255, 80, 80);       // Soft red
const C_DIM: Color = Color::Rgb(120, 120, 150);      // Muted grey-purple
const C_TEXT: Color = Color::Rgb(220, 220, 235);     // Near-white text
const C_BG: Color = Color::Reset;                    // Terminal background

// ── Helper: bytes → human-readable string ────────────────────────────────────

pub fn fmt_bytes(bytes: u64) -> String {
    const GIB: u64 = 1_073_741_824;
    const MIB: u64 = 1_048_576;
    const KIB: u64 = 1_024;
    if bytes >= GIB {
        format!("{:.2} GB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.0} MB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.0} KB", bytes as f64 / KIB as f64)
    } else {
        format!("{bytes} B")
    }
}

// ── Health → colour ──────────────────────────────────────────────────────────

fn health_color(h: &Health) -> Color {
    match h {
        Health::Normal => C_OK,
        Health::Pressure => C_WARN,
        Health::Critical => C_CRIT,
    }
}

fn pct_color(pct: f64, warn: f64, crit: f64) -> Color {
    if pct >= crit {
        C_CRIT
    } else if pct >= warn {
        C_WARN
    } else {
        C_OK
    }
}

fn ratio_color(ratio: f64) -> Color {
    if ratio < RATIO_THRESHOLD {
        C_CRIT
    } else if ratio < 2.0 {
        C_WARN
    } else {
        C_OK
    }
}

// ── Alert status helpers ──────────────────────────────────────────────────────

fn alert_status_spans(active: bool, label: &str) -> Line<'static> {
    let (icon, color) = if active {
        ("🔴 ALERT", C_CRIT)
    } else {
        ("✅ OK   ", C_OK)
    };
    Line::from(vec![
        Span::styled(format!("  {label:<18}"), Style::default().fg(C_DIM)),
        Span::styled(icon, Style::default().fg(color).add_modifier(Modifier::BOLD)),
    ])
}

// ── Sparkline data conversion ─────────────────────────────────────────────────

/// Scale f64 values to u64 for the Sparkline widget.
/// ratio_history: values are e.g. 0.0–5.0, scale by 100
fn ratio_to_u64(v: f64) -> u64 {
    (v * 100.0).round() as u64
}

// ── Main draw function ────────────────────────────────────────────────────────

#[allow(dead_code)]
pub struct AppState<'a> {
    pub zram: &'a ZramStats,
    pub ram: &'a RamStats,
    pub alerts: &'a AlertState,
    pub swap_entries: &'a Vec<SwapEntry>,
    pub swap_totals: &'a SwapTotals,
    pub usage_history: &'a VecDeque<f64>,
    pub ratio_history: &'a VecDeque<f64>,
    pub session_peak_pct: f64,
    pub log_path: &'a str,
    pub paused: bool,
    pub tick_count: u64,
}

pub fn draw(f: &mut Frame, state: &AppState) {
    let area = f.area();

    // ── Root vertical split: header / body / footer ──
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // header
            Constraint::Min(0),     // body
            Constraint::Length(3),  // footer
        ])
        .split(area);

    draw_header(f, root[0], state);
    draw_body(f, root[1], state);
    draw_footer(f, root[2], state);
}

// ── Header ────────────────────────────────────────────────────────────────────

fn draw_header(f: &mut Frame, area: Rect, state: &AppState) {
    let pause_label = if state.paused { "  ⏸ PAUSED" } else { "" };
    let title = format!("  ⚡ Z-SIGHT  ·  ZRAM Monitor{pause_label}");
    let right = format!("Zorin OS  ·  {}  RAM  ", fmt_bytes(state.ram.total));

    let header_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(C_BRAND))
        .style(Style::default().bg(C_BG));

    let inner = header_block.inner(area);
    f.render_widget(header_block, area);

    // Split inner area: title left, system info right
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(right.len() as u16 + 2)])
        .split(inner);

    f.render_widget(
        Paragraph::new(title)
            .style(Style::default().fg(C_BRAND).add_modifier(Modifier::BOLD)),
        cols[0],
    );
    f.render_widget(
        Paragraph::new(right)
            .style(Style::default().fg(C_DIM))
            .alignment(Alignment::Right),
        cols[1],
    );
}

// ── Body ──────────────────────────────────────────────────────────────────────

fn draw_body(f: &mut Frame, area: Rect, state: &AppState) {
    // Left column (40%) | Right column (60%)
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
        .split(area);

    draw_left_column(f, cols[0], state);
    draw_right_column(f, cols[1], state);
}

// ── Left column: ZRAM stats + Alert panel ────────────────────────────────────

fn draw_left_column(f: &mut Frame, area: Rect, state: &AppState) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(9)])
        .split(area);

    draw_zram_panel(f, rows[0], state);
    draw_alert_panel(f, rows[1], state);
}

fn draw_zram_panel(f: &mut Frame, area: Rect, state: &AppState) {
    let zs = state.zram;
    let health_col = health_color(&zs.health);

    let block = Block::default()
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("ZRAM DEVICE: zram0", Style::default().fg(C_BRAND).add_modifier(Modifier::BOLD)),
            Span::raw(" "),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(C_BRAND));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Split inner: gauge row + stats rows
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // spacer
            Constraint::Length(1), // "Usage" label
            Constraint::Length(3), // gauge
            Constraint::Length(1), // spacer
            Constraint::Min(0),    // stat lines
        ])
        .split(inner);

    // Usage label
    f.render_widget(
        Paragraph::new(Span::styled(
            "  Usage",
            Style::default().fg(C_DIM).add_modifier(Modifier::BOLD),
        )),
        rows[1],
    );

    // Gauge
    let gauge_color = pct_color(zs.usage_pct, 70.0, 85.0);
    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::NONE))
        .gauge_style(
            Style::default()
                .fg(gauge_color)
                .bg(Color::Rgb(40, 40, 55))
                .add_modifier(Modifier::BOLD),
        )
        .percent(zs.usage_pct.clamp(0.0, 100.0) as u16)
        .label(format!("{:.1}%", zs.usage_pct));
    f.render_widget(gauge, rows[2]);

    // Stat lines
    let ratio_col = ratio_color(zs.compression_ratio);
    let stats = vec![
        Line::from(vec![
            Span::styled("  Disksize   ", Style::default().fg(C_DIM)),
            Span::styled(fmt_bytes(zs.disksize), Style::default().fg(C_TEXT)),
        ]),
        Line::from(vec![
            Span::styled("  Orig Data  ", Style::default().fg(C_DIM)),
            Span::styled(fmt_bytes(zs.orig_data_size), Style::default().fg(C_TEXT)),
        ]),
        Line::from(vec![
            Span::styled("  Compr Data ", Style::default().fg(C_DIM)),
            Span::styled(fmt_bytes(zs.compr_data_size), Style::default().fg(C_TEXT)),
        ]),
        Line::from(vec![
            Span::styled("  Overhead   ", Style::default().fg(C_DIM)),
            Span::styled(fmt_bytes(zs.mem_used_total), Style::default().fg(C_TEXT)),
        ]),
        Line::from(vec![
            Span::styled("  Ratio      ", Style::default().fg(C_DIM)),
            Span::styled(
                format!("{:.2}x", zs.compression_ratio),
                Style::default().fg(ratio_col).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Health     ", Style::default().fg(C_DIM)),
            Span::styled(
                zs.health.label(),
                Style::default().fg(health_col).add_modifier(Modifier::BOLD),
            ),
        ]),
    ];

    f.render_widget(
        Paragraph::new(stats).wrap(Wrap { trim: false }),
        rows[4],
    );
}

fn draw_alert_panel(f: &mut Frame, area: Rect, state: &AppState) {
    let block = Block::default()
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("ALERT STATUS", Style::default().fg(C_BRAND).add_modifier(Modifier::BOLD)),
            Span::raw(" "),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(C_DIM));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines = vec![
        Line::from(""),
        alert_status_spans(
            state.alerts.usage_alert_active(),
            &format!("Usage > {USAGE_THRESHOLD}%"),
        ),
        alert_status_spans(
            state.alerts.ratio_alert_active(),
            &format!("Ratio < {RATIO_THRESHOLD}x"),
        ),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Last alert: ", Style::default().fg(C_DIM)),
            Span::styled(
                state.alerts.last_alert_display(),
                Style::default().fg(C_TEXT),
            ),
        ]),
    ];

    f.render_widget(Paragraph::new(lines), inner);
}

// ── Right column: System RAM + sparkline charts ───────────────────────────────

fn draw_right_column(f: &mut Frame, area: Rect, state: &AppState) {
    // Height for swap panel: 2 (border+title) + 1 (spacer) + entries × 3 (label+gauge+spacer) + 1 (total line)
    let swap_h = (2 + 1 + state.swap_entries.len() as u16 * 3 + 2).max(6);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),         // System RAM panel
            Constraint::Length(swap_h),    // Swap devices panel
            Constraint::Min(0),            // Sparkline charts
        ])
        .split(area);

    draw_system_ram_panel(f, rows[0], state);
    draw_swap_panel(f, rows[1], state);
    draw_charts(f, rows[2], state);
}

fn draw_system_ram_panel(f: &mut Frame, area: Rect, state: &AppState) {
    let rs = state.ram;
    let ram_col = pct_color(rs.used_pct, 60.0, 80.0);

    let block = Block::default()
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("SYSTEM MEMORY", Style::default().fg(C_BRAND).add_modifier(Modifier::BOLD)),
            Span::raw(" "),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(C_BRAND));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1), // label
            Constraint::Length(2), // gauge
            Constraint::Min(0),    // stats
        ])
        .split(inner);

    f.render_widget(
        Paragraph::new(Span::styled(
            "  RAM Used",
            Style::default().fg(C_DIM).add_modifier(Modifier::BOLD),
        )),
        rows[1],
    );

    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::NONE))
        .gauge_style(
            Style::default()
                .fg(ram_col)
                .bg(Color::Rgb(40, 40, 55))
                .add_modifier(Modifier::BOLD),
        )
        .percent(rs.used_pct.clamp(0.0, 100.0) as u16)
        .label(format!("{:.1}%  ({} / {})", rs.used_pct, fmt_bytes(rs.used), fmt_bytes(rs.total)));
    f.render_widget(gauge, rows[2]);

    let stats = vec![
        Line::from(vec![
            Span::styled("  Total     ", Style::default().fg(C_DIM)),
            Span::styled(fmt_bytes(rs.total), Style::default().fg(C_TEXT)),
            Span::styled("    Available  ", Style::default().fg(C_DIM)),
            Span::styled(fmt_bytes(rs.available), Style::default().fg(C_TEXT)),
        ]),
    ];
    f.render_widget(Paragraph::new(stats), rows[3]);
}

// ── Swap devices panel ────────────────────────────────────────────────────────

fn draw_swap_panel(f: &mut Frame, area: Rect, state: &AppState) {
    let totals = state.swap_totals;
    let total_col = pct_color(totals.used_pct, 60.0, 80.0);

    let block = Block::default()
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled(
                "SWAP DEVICES",
                Style::default().fg(C_BRAND).add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                format!(
                    "total {:.1}%  ({} / {})",
                    totals.used_pct,
                    fmt_bytes(totals.total_used),
                    fmt_bytes(totals.total_size)
                ),
                Style::default().fg(total_col),
            ),
            Span::raw(" "),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(C_BRAND));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if state.swap_entries.is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled(
                "  No active swap devices found.",
                Style::default().fg(C_DIM),
            )),
            inner,
        );
        return;
    }

    // One row group per entry: [label row] + [gauge row] + [spacer]
    let entry_count = state.swap_entries.len();
    let mut constraints: Vec<Constraint> = Vec::new();
    for _ in 0..entry_count {
        constraints.push(Constraint::Length(1)); // label
        constraints.push(Constraint::Length(2)); // gauge
        constraints.push(Constraint::Length(1)); // gap
    }
    constraints.push(Constraint::Min(0)); // absorb leftover

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    for (i, entry) in state.swap_entries.iter().enumerate() {
        let label_idx = i * 3;
        let gauge_idx = label_idx + 1;

        let col = pct_color(entry.used_pct, 60.0, 80.0);
        let kind_label = if entry.kind == "partition" { "partition" } else { "file     " };
        let prio_str = if entry.priority >= 0 {
            format!("prio +{}", entry.priority)
        } else {
            format!("prio {}", entry.priority)
        };

        // Label row: name  [type]  prio N
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(
                    format!("{:<16}", entry.display_name),
                    Style::default().fg(C_TEXT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("[{kind_label}]"),
                    Style::default().fg(C_DIM),
                ),
                Span::styled(
                    format!("  {prio_str}"),
                    Style::default().fg(C_DIM),
                ),
            ])),
            rows[label_idx],
        );

        // Gauge row
        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::NONE))
            .gauge_style(
                Style::default()
                    .fg(col)
                    .bg(Color::Rgb(40, 40, 55))
                    .add_modifier(Modifier::BOLD),
            )
            .percent(entry.used_pct.clamp(0.0, 100.0) as u16)
            .label(format!(
                "{:.1}%  ({} / {})",
                entry.used_pct,
                fmt_bytes(entry.used),
                fmt_bytes(entry.size),
            ));
        f.render_widget(gauge, rows[gauge_idx]);
    }
}

fn draw_charts(f: &mut Frame, area: Rect, state: &AppState) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    draw_ratio_chart(f, cols[0], state);
    draw_usage_chart(f, cols[1], state);
}

fn draw_ratio_chart(f: &mut Frame, area: Rect, state: &AppState) {
    let current = state.zram.compression_ratio;
    let col = ratio_color(current);

    let block = Block::default()
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled(
                format!("RATIO HISTORY  {current:.2}x"),
                Style::default().fg(col).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(C_DIM));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Convert history to u64 for sparkline (scale ×100)
    let data: Vec<u64> = state.ratio_history.iter().map(|&v| ratio_to_u64(v)).collect();

    let sparkline = Sparkline::default()
        .block(Block::default().borders(Borders::NONE))
        .data(&data)
        .style(Style::default().fg(col))
        .bar_set(symbols::bar::NINE_LEVELS);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(inner);

    // Threshold label
    f.render_widget(
        Paragraph::new(Span::styled(
            format!("  · · · · · · · · · · · · {RATIO_THRESHOLD}x limit · ·"),
            Style::default().fg(C_DIM),
        )),
        rows[0],
    );

    f.render_widget(sparkline, rows[1]);
}

fn draw_usage_chart(f: &mut Frame, area: Rect, state: &AppState) {
    let current = state.zram.usage_pct;
    let col = pct_color(current, 70.0, 85.0);

    let block = Block::default()
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled(
                format!("USAGE HISTORY  {current:.1}%"),
                Style::default().fg(col).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(C_DIM));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let data: Vec<u64> = state.usage_history.iter().map(|&v| v.round() as u64).collect();

    let sparkline = Sparkline::default()
        .block(Block::default().borders(Borders::NONE))
        .data(&data)
        .style(Style::default().fg(col))
        .bar_set(symbols::bar::NINE_LEVELS);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(inner);

    f.render_widget(
        Paragraph::new(Span::styled(
            format!("  · · · · · · · · · · · · {USAGE_THRESHOLD}% limit · "),
            Style::default().fg(C_DIM),
        )),
        rows[0],
    );

    f.render_widget(sparkline, rows[1]);
}

// ── Footer ────────────────────────────────────────────────────────────────────

fn draw_footer(f: &mut Frame, area: Rect, state: &AppState) {
    let peak_col = pct_color(state.session_peak_pct, 70.0, 85.0);

    let left = format!(
        "  [q] Quit   [p] {}",
        if state.paused { "Resume" } else { "Pause " }
    );
    let right = format!(
        "Peak: {:.1}%  ·  Log: {}  ",
        state.session_peak_pct,
        state.log_path
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(C_DIM));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(right.len() as u16 + 2)])
        .split(inner);

    f.render_widget(
        Paragraph::new(left).style(Style::default().fg(C_DIM)),
        cols[0],
    );
    f.render_widget(
        Paragraph::new(vec![Line::from(vec![
            Span::styled("Peak: ", Style::default().fg(C_DIM)),
            Span::styled(
                format!("{:.1}%", state.session_peak_pct),
                Style::default().fg(peak_col).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  ·  Log: {}  ", state.log_path),
                Style::default().fg(C_DIM),
            ),
        ])])
        .alignment(Alignment::Right),
        cols[1],
    );
}
