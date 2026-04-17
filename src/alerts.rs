/// Desktop alert system using notify-rust with per-alert cooldowns.
use std::time::{Duration, Instant};
use notify_rust::Notification;

/// Thresholds — change here if you want different limits.
pub const USAGE_THRESHOLD: f64 = 85.0;
pub const RATIO_THRESHOLD: f64 = 1.5;

/// Minimum time between the same alert firing again.
const COOLDOWN: Duration = Duration::from_secs(60);

/// Which alert condition was triggered.
#[derive(Debug, Clone, PartialEq)]
pub enum AlertKind {
    HighUsage,
    LowRatio,
}

/// A record of an alert that has fired.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AlertEvent {
    pub kind: AlertKind,
    pub fired_at: Instant,
}

/// Tracks the last time each alert was dispatched to enforce cooldowns.
pub struct AlertState {
    last_usage_alert: Option<Instant>,
    last_ratio_alert: Option<Instant>,
    /// Most recent alert event, exposed to the TUI.
    pub last_event: Option<AlertEvent>,
}

impl AlertState {
    pub fn new() -> Self {
        AlertState {
            last_usage_alert: None,
            last_ratio_alert: None,
            last_event: None,
        }
    }

    /// Check metrics and fire notifications if thresholds are breached and
    /// the cooldown period has elapsed. Returns the fired event if any.
    pub fn check(&mut self, usage_pct: f64, compression_ratio: f64) -> Option<AlertKind> {
        let now = Instant::now();

        if usage_pct > USAGE_THRESHOLD {
            let should_fire = self
                .last_usage_alert
                .map(|t| now.duration_since(t) >= COOLDOWN)
                .unwrap_or(true);

            if should_fire {
                let _ = Notification::new()
                    .summary("⚡ Z-Sight: ZRAM Pressure")
                    .body(&format!(
                        "ZRAM usage is {usage_pct:.1}% (threshold: {USAGE_THRESHOLD}%)"
                    ))
                    .icon("dialog-warning")
                    .show();

                self.last_usage_alert = Some(now);
                let event = AlertEvent {
                    kind: AlertKind::HighUsage,
                    fired_at: now,
                };
                self.last_event = Some(event);
                return Some(AlertKind::HighUsage);
            }
        }

        if compression_ratio < RATIO_THRESHOLD && compression_ratio > 0.0 {
            let should_fire = self
                .last_ratio_alert
                .map(|t| now.duration_since(t) >= COOLDOWN)
                .unwrap_or(true);

            if should_fire {
                let _ = Notification::new()
                    .summary("⚡ Z-Sight: Low Compression")
                    .body(&format!(
                        "Compression ratio is {compression_ratio:.2}x (threshold: {RATIO_THRESHOLD}x) — uncompressible memory load detected."
                    ))
                    .icon("dialog-warning")
                    .show();

                self.last_ratio_alert = Some(now);
                let event = AlertEvent {
                    kind: AlertKind::LowRatio,
                    fired_at: now,
                };
                self.last_event = Some(event);
                return Some(AlertKind::LowRatio);
            }
        }

        None
    }

    /// Human-readable time since last alert, for TUI display.
    pub fn last_alert_display(&self) -> String {
        match &self.last_event {
            None => "--".to_string(),
            Some(ev) => {
                let secs = ev.fired_at.elapsed().as_secs();
                if secs < 60 {
                    format!("{secs}s ago")
                } else {
                    format!("{}m ago", secs / 60)
                }
            }
        }
    }

    /// Whether the high-usage alert is in an active (recently fired) state.
    pub fn usage_alert_active(&self) -> bool {
        self.last_usage_alert
            .map(|t| t.elapsed() < COOLDOWN)
            .unwrap_or(false)
    }

    /// Whether the low-ratio alert is in an active (recently fired) state.
    pub fn ratio_alert_active(&self) -> bool {
        self.last_ratio_alert
            .map(|t| t.elapsed() < COOLDOWN)
            .unwrap_or(false)
    }
}
