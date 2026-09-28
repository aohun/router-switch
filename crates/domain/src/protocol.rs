use crate::AppKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RequestProtocol {
    Anthropic,
    #[serde(rename = "openai-chat")]
    OpenAiChat,
    #[serde(rename = "openai-responses")]
    OpenAiResponses,
}

impl RequestProtocol {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::OpenAiChat => "openai-chat",
            Self::OpenAiResponses => "openai-responses",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Anthropic => "Anthropic Messages (/v1/messages)",
            Self::OpenAiChat => "Chat Completions (/chat/completions)",
            Self::OpenAiResponses => "Responses (/responses)",
        }
    }

    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "anthropic" | "claude" | "messages" => Self::Anthropic,
            "openai-responses" | "responses" => Self::OpenAiResponses,
            "openai-chat" | "openai" | "chat" | "chat-completions" => Self::OpenAiChat,
            _ => Self::OpenAiChat,
        }
    }

    pub fn all() -> [Self; 3] {
        [Self::Anthropic, Self::OpenAiChat, Self::OpenAiResponses]
    }

    pub fn default_for_app(app: AppKind) -> Self {
        match app {
            AppKind::Claude | AppKind::ZCode => Self::Anthropic,
            AppKind::Codex => Self::OpenAiResponses,
            _ => Self::OpenAiChat,
        }
    }

    pub fn picker_badge(self) -> &'static str {
        match self {
            Self::Anthropic => "ANTHROPIC",
            Self::OpenAiChat | Self::OpenAiResponses => "OPENAI",
        }
    }

    /// AstrLink capability protocol id for this ingress dialect.
    pub fn capability_id(self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic.messages",
            Self::OpenAiChat => "openai.chat",
            Self::OpenAiResponses => "openai.responses",
        }
    }

    pub fn from_capability_id(id: &str) -> Option<Self> {
        match id.trim() {
            "anthropic.messages" => Some(Self::Anthropic),
            "openai.chat" | "openai.completions" => Some(Self::OpenAiChat),
            "openai.responses" | "openai.responses.compact" => Some(Self::OpenAiResponses),
            _ => None,
        }
    }
}

pub const THINKING_EFFORTS: [(&str, &str); 5] = [
    ("low", "Low"),
    ("medium", "Medium"),
    ("high", "High"),
    ("xhigh", "Extra High"),
    ("max", "Max"),
];

pub const DEFAULT_THINKING_EFFORT: &str = "high";

pub fn normalize_thinking_effort(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "low" => "low".into(),
        "medium" => "medium".into(),
        "xhigh" | "extra-high" | "extra_high" | "extra high" => "xhigh".into(),
        "max" => "max".into(),
        _ => DEFAULT_THINKING_EFFORT.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_aliases() {
        assert_eq!(RequestProtocol::parse("claude"), RequestProtocol::Anthropic);
        assert_eq!(
            RequestProtocol::parse("responses"),
            RequestProtocol::OpenAiResponses
        );
        assert_eq!(RequestProtocol::parse(""), RequestProtocol::OpenAiChat);
    }

    #[test]
    fn normalizes_effort() {
        assert_eq!(normalize_thinking_effort("Extra High"), "xhigh");
        assert_eq!(normalize_thinking_effort(""), "high");
    }
}
