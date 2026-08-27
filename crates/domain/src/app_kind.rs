use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AppKind {
    Codex,
    Claude,
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
            "grok" => Some(Self::Grok),
            "opencode" => Some(Self::OpenCode),
            "pi" => Some(Self::Pi),
            "cursor" => Some(Self::Cursor),
            "zcode" => Some(Self::ZCode),
            "workbuddy" | "work_buddy" | "codebuddy" | "code_buddy" => Some(Self::WorkBuddy),
            _ => None,
        }
    }
}
