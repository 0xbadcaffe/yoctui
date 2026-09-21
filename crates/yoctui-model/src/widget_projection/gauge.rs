#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetRole {
    Primary,
    Success,
    Warning,
    Error,
    Running,
    Pending,
    Disabled,
    Accent,
    Muted,
    Informational,
    Progress,
    Cpu,
    Memory,
    DiskRead,
    DiskWrite,
    NetworkRx,
    NetworkTx,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetState {
    Available,
    Active,
    Empty,
    Unknown,
    Unavailable,
    Partial,
    TerminalSuccess,
    TerminalFailure,
    TerminalCancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetTerminalState {
    Success,
    Failure,
    Cancelled,
}

impl WidgetTerminalState {
    const fn widget_state(self) -> WidgetState {
        match self {
            Self::Success => WidgetState::TerminalSuccess,
            Self::Failure => WidgetState::TerminalFailure,
            Self::Cancelled => WidgetState::TerminalCancelled,
        }
    }

    const fn role(self) -> WidgetRole {
        match self {
            Self::Success => WidgetRole::Success,
            Self::Failure => WidgetRole::Error,
            Self::Cancelled => WidgetRole::Warning,
        }
    }
}

impl WidgetState {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Active => "active",
            Self::Empty => "empty",
            Self::Unknown => "unknown",
            Self::Unavailable => "unavailable",
            Self::Partial => "partial",
            Self::TerminalSuccess => "succeeded",
            Self::TerminalFailure => "failed",
            Self::TerminalCancelled => "cancelled",
        }
    }

    pub const fn marker(self, unicode: bool, reduced_motion: bool) -> &'static str {
        match self {
            Self::Available => {
                if unicode {
                    "◆"
                } else {
                    "+"
                }
            }
            Self::Active if unicode && !reduced_motion => "…",
            Self::Active => ">",
            Self::Empty => {
                if unicode {
                    "∅"
                } else {
                    "-"
                }
            }
            Self::Unknown => "?",
            Self::Unavailable => "!",
            Self::Partial => "!",
            Self::TerminalSuccess => {
                if unicode {
                    "✓"
                } else {
                    "+"
                }
            }
            Self::TerminalFailure => {
                if unicode {
                    "✕"
                } else {
                    "x"
                }
            }
            Self::TerminalCancelled => {
                if unicode {
                    "■"
                } else {
                    "#"
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundedFraction {
    pub current: u64,
    pub total: u64,
}

impl BoundedFraction {
    pub const fn new(current: u64, total: u64) -> Option<Self> {
        if total == 0 {
            None
        } else {
            Some(Self { current, total })
        }
    }

    pub fn percent(self) -> u8 {
        let percent = u128::from(self.current)
            .saturating_mul(100)
            .checked_div(u128::from(self.total))
            .unwrap_or(0)
            .min(100);
        u8::try_from(percent).unwrap_or(100)
    }

    pub fn ratio(self) -> f64 {
        (self.current.min(self.total) as f64 / self.total as f64).clamp(0.0, 1.0)
    }

    pub fn exact_text(self) -> String {
        format!("{}/{} ({}%)", self.current, self.total, self.percent())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GaugeProjection {
    pub label: String,
    pub state: WidgetState,
    pub role: WidgetRole,
    pub fraction: Option<BoundedFraction>,
    pub detail: Option<String>,
}

impl GaugeProjection {
    pub fn determinate(
        label: impl Into<String>,
        current: u64,
        total: u64,
        role: WidgetRole,
    ) -> Self {
        let label = label.into();
        match BoundedFraction::new(current, total) {
            Some(fraction) if current <= total => Self {
                label,
                state: WidgetState::Available,
                role,
                fraction: Some(fraction),
                detail: None,
            },
            Some(fraction) => Self {
                label,
                state: WidgetState::Partial,
                role: WidgetRole::Warning,
                fraction: Some(fraction),
                detail: Some("reported value exceeds total".into()),
            },
            None => Self::explicit(label, WidgetState::Unknown, role, "total not reported"),
        }
    }

    pub fn indeterminate(label: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::explicit(label, WidgetState::Active, WidgetRole::Running, detail)
    }

    pub fn explicit(
        label: impl Into<String>,
        state: WidgetState,
        role: WidgetRole,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            label: label.into(),
            state,
            role,
            fraction: None,
            detail: Some(detail.into()).filter(|detail| !detail.is_empty()),
        }
    }

    pub fn terminal(
        label: impl Into<String>,
        current: u64,
        total: u64,
        terminal: WidgetTerminalState,
        detail: impl Into<String>,
    ) -> Self {
        let role = terminal.role();
        if total == 0 {
            let detail = detail.into();
            return Self {
                label: label.into(),
                state: terminal.widget_state(),
                role,
                fraction: None,
                detail: Some(if detail.is_empty() {
                    format!("{current}/?")
                } else {
                    format!("{current}/? · {detail}")
                }),
            };
        }
        let mut projection = Self::determinate(label, current, total, role);
        if projection.fraction.is_some() {
            projection.state = terminal.widget_state();
            projection.role = role;
            projection.detail = Some(detail.into()).filter(|detail| !detail.is_empty());
        }
        projection
    }

    pub fn text(&self, unicode: bool, reduced_motion: bool) -> String {
        let mut parts = vec![format!(
            "{} {}",
            self.state.marker(unicode, reduced_motion),
            self.label
        )];
        if self.state != WidgetState::Available {
            parts.push(self.state.label().into());
        }
        if let Some(fraction) = self.fraction {
            parts.push(fraction.exact_text());
        }
        if let Some(detail) = &self.detail {
            parts.push(detail.clone());
        }
        parts.join(" · ")
    }
}

