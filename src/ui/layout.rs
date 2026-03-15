//! Layout management: defines layout modes and computes panel areas.

use ratatui::layout::{Constraint, Direction, Layout, Rect};

// ── Layout modes ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    /// All panels visible with balanced sizing.
    Detailed,
    /// Reduced panel sizes, more data density.
    Compact,
    /// Single panel fullscreen.
    Focus,
    /// Only the process list.
    ProcessOnly,
}

impl LayoutMode {
    pub fn next(self) -> Self {
        match self {
            Self::Detailed => Self::Compact,
            Self::Compact => Self::ProcessOnly,
            Self::ProcessOnly => Self::Focus,
            Self::Focus => Self::Detailed,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Detailed => "Detailed",
            Self::Compact => "Compact",
            Self::Focus => "Focus",
            Self::ProcessOnly => "Process",
        }
    }
}

// ── Active view (which panel is focused) ─────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveView {
    Overview,
    Network,
    Disk,
    Gpu,
    History,
    Remote,
    /// 2-D heat map of per-core CPU utilisation over time.
    Heatmap,
    /// Ring-buffered log of system anomaly events.
    Alerts,
    /// Docker/Podman container monitor.
    Containers,
    /// System fan speeds and control status.
    Fans,
}

impl ActiveView {
    pub fn label(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Network => "Network",
            Self::Disk => "Disk",
            Self::Gpu => "GPU",
            Self::History => "History",
            Self::Remote => "Remote",
            Self::Heatmap => "Heatmap",
            Self::Alerts => "Alerts",
            Self::Containers => "Containers",
            Self::Fans => "Fans",
        }
    }
}

// ── Layout computation ───────────────────────────────────────────────────────

/// Computed layout areas for the main panels.
pub struct PanelAreas {
    pub header: Rect,
    pub top_left: Rect,
    pub top_right: Rect,
    pub bottom_left: Rect,
    pub bottom_right: Rect,
    pub footer: Rect,
}

/// Compute panel areas for the Detailed layout.
pub fn compute_detailed(area: Rect) -> PanelAreas {
    let main = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // header
            Constraint::Min(12),   // top row
            Constraint::Min(10),   // bottom row
            Constraint::Length(1), // footer
        ])
        .split(area);

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(main[1]);

    let bottom = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(main[2]);

    PanelAreas {
        header: main[0],
        top_left: top[0],
        top_right: top[1],
        bottom_left: bottom[0],
        bottom_right: bottom[1],
        footer: main[3],
    }
}

/// Compute panel areas for the Compact layout.
pub fn compute_compact(area: Rect) -> PanelAreas {
    let main = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),  // header (smaller)
            Constraint::Min(8),    // top row
            Constraint::Min(8),    // bottom row
            Constraint::Length(1), // footer
        ])
        .split(area);

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main[1]);

    let bottom = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(main[2]);

    PanelAreas {
        header: main[0],
        top_left: top[0],
        top_right: top[1],
        bottom_left: bottom[0],
        bottom_right: bottom[1],
        footer: main[3],
    }
}

/// For ProcessOnly mode, single fullscreen area with header/footer.
pub fn compute_process_only(area: Rect) -> PanelAreas {
    let main = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(1),
        ])
        .split(area);

    PanelAreas {
        header: main[0],
        top_left: Rect::default(),
        top_right: Rect::default(),
        bottom_left: Rect::default(),
        bottom_right: main[1],
        footer: main[2],
    }
}

/// For Focus mode, a single panel takes the entire center area.
pub fn compute_focus(area: Rect) -> PanelAreas {
    let main = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(1),
        ])
        .split(area);

    PanelAreas {
        header: main[0],
        top_left: main[1],
        top_right: Rect::default(),
        bottom_left: Rect::default(),
        bottom_right: Rect::default(),
        footer: main[2],
    }
}
