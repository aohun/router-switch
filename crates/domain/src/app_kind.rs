use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AppKind {
    Codex,
    Claude,
    ClaudeDesktop,
    Grok,
    OpenCode,
    Pi,
    Cursor,
    ZCode,
    WorkBuddy,
}

impl AppKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Claude => "claude",
            Self::ClaudeDesktop => "claude-desktop",
            Self::Grok => "grok",
            Self::OpenCode => "opencode",
            Self::Pi => "pi",
            Self::Cursor => "cursor",
            Self::ZCode => "zcode",
            Self::WorkBuddy => "workbuddy",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Codex => "Codex",
            Self::Claude => "Claude Code",
            Self::ClaudeDesktop => "Claude Desktop",
            Self::Grok => "Grok Build",
            Self::OpenCode => "OpenCode",
            Self::Pi => "Pi",
            Self::Cursor => "Cursor",
            Self::ZCode => "ZCode",
            Self::WorkBuddy => "WorkBuddy",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "codex" => Some(Self::Codex),
            "claude" => Some(Self::Claude),
            "claude-desktop" | "claude_desktop" | "claudedesktop" => Some(Self::ClaudeDesktop),
            "grok" => Some(Self::Grok),
            "opencode" => Some(Self::OpenCode),
            "pi" => Some(Self::Pi),
            "cursor" => Some(Self::Cursor),
            "zcode" => Some(Self::ZCode),
            "workbuddy" | "work_buddy" | "codebuddy" | "code_buddy" => Some(Self::WorkBuddy),
            _ => None,
        }
    }

    /// 全部应用(供遍历加载等场景使用，避免硬编码漏项)
    pub const ALL: &'static [AppKind] = &[
        AppKind::Codex,
        AppKind::Claude,
        AppKind::ClaudeDesktop,
        AppKind::Grok,
        AppKind::OpenCode,
        AppKind::Pi,
        AppKind::Cursor,
        AppKind::ZCode,
        AppKind::WorkBuddy,
    ];
}
