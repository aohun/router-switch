use serde::{Deserialize, Serialize};

use crate::{
    AppKind, ClaudeSettings, CodexSettings, CursorSettings, GrokSettings, OpenCodeSettings,
    PiSettings, WorkBuddySettings, ZCodeSettings,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Provider {
    pub id: String,
    pub app: AppKind,
    pub name: String,
    pub website_url: Option<String>,
    pub settings: ProviderSettings,
    pub created_at: i64,
    pub sort_index: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ProviderSettings {
    Codex(CodexSettings),
    Claude(ClaudeSettings),
    Grok(GrokSettings),
    OpenCode(OpenCodeSettings),
    Pi(PiSettings),
    Cursor(CursorSettings),
    ZCode(ZCodeSettings),
    WorkBuddy(WorkBuddySettings),
    Unsupported { app: AppKind },
}

impl Provider {
    pub fn codex_settings(&self) -> Option<&CodexSettings> {
        match &self.settings {
            ProviderSettings::Codex(settings) => Some(settings),
            _ => None,
        }
    }

    pub fn claude_settings(&self) -> Option<&ClaudeSettings> {
        match &self.settings {
            ProviderSettings::Claude(settings) => Some(settings),
            _ => None,
        }
    }

    pub fn grok_settings(&self) -> Option<&GrokSettings> {
        match &self.settings {
            ProviderSettings::Grok(settings) => Some(settings),
            _ => None,
        }
    }

    pub fn opencode_settings(&self) -> Option<&OpenCodeSettings> {
        match &self.settings {
            ProviderSettings::OpenCode(settings) => Some(settings),
            _ => None,
        }
    }

    pub fn pi_settings(&self) -> Option<&PiSettings> {
        match &self.settings {
            ProviderSettings::Pi(settings) => Some(settings),
            _ => None,
        }
    }

    pub fn cursor_settings(&self) -> Option<&CursorSettings> {
        match &self.settings {
            ProviderSettings::Cursor(settings) => Some(settings),
            _ => None,
        }
    }

    pub fn zcode_settings(&self) -> Option<&ZCodeSettings> {
        match &self.settings {
            ProviderSettings::ZCode(settings) => Some(settings),
            _ => None,
        }
    }

    pub fn workbuddy_settings(&self) -> Option<&WorkBuddySettings> {
        match &self.settings {
            ProviderSettings::WorkBuddy(settings) => Some(settings),
            _ => None,
        }
    }

    pub fn is_official(&self) -> bool {
        match &self.settings {
            ProviderSettings::Codex(s) => s.kind.is_official(),
            ProviderSettings::Claude(s) => s.kind.is_official(),
            ProviderSettings::Grok(s) => s.kind.is_official(),
            ProviderSettings::OpenCode(s) => s.kind.is_official(),
            ProviderSettings::Pi(s) => s.kind.is_official(),
            ProviderSettings::Cursor(s) => s.kind.is_official(),
            ProviderSettings::ZCode(s) => s.kind.is_official(),
            ProviderSettings::WorkBuddy(s) => s.kind.is_official(),
            ProviderSettings::Unsupported { .. } => false,
        }
    }

    pub fn is_official_codex(&self) -> bool {
        self.codex_settings()
            .is_some_and(|settings| settings.kind.is_official())
    }
}

pub fn new_provider_id(name: &str) -> String {
    let slug = slugify(name);
    let millis = unix_millis();
    if slug.is_empty() {
        format!("provider-{millis}")
    } else {
        format!("{slug}-{millis}")
    }
}

fn slugify(name: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

fn unix_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}
