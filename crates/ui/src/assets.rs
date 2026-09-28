//! App assets: UI SVG icons + Wake-style colorful brand PNGs.
//!
//! Brand icons must be rendered with [`brand_img`] / `gpui::img`, never via
//! `Icon` + `text_color` — otherwise brand colors get tinted away.

use gpui::{img, px, AbsoluteLength, AssetSource, Img, SharedString, Styled};
use gpui_component::IconNamed;
use std::borrow::Cow;

pub struct AppAssets;

/// Agent brand icons (color PNG with alpha). Paths match Wake's `brands/*.png`.
macro_rules! brands {
    ($($name:literal),* $(,)?) => {
        fn lookup_brand(path: &str) -> Option<&'static [u8]> {
            match path {
                $(concat!("brands/", $name, ".png") =>
                    Some(include_bytes!(concat!("../assets/brands/", $name, ".png"))),)*
                _ => None,
            }
        }
    };
}

brands!(
    "claude-code",
    "codex",
    "cursor",
    "cursor-light",
    "opencode",
    "opencode-light",
    "pi",
    "pi-light",
    "grok",
    "grok-light",
    "kimi",
    "kimi-light",
    "gemini",
    "antigravity",
    "qoder",
    "qoder-light",
    "openclaw",
    "workbuddy",
    "zcode",
    "deepseek",
    "omp",
    "hermes",
    "hermes-light",
    "cc-switch",
);

/// Resolve brand PNG/SVG path for a tool / app id (aligns with Wake `AgentId::brand_icon`).
pub fn brand_icon_path(app_id: &str, dark: bool) -> &'static str {
    match app_id {
        "claude" | "claude-desktop" | "claude-code" | "anthropic" => "brands/claude-code.png",
        "codex" | "openai" => "brands/codex.png",
        "cursor" => {
            if dark {
                "brands/cursor.png"
            } else {
                "brands/cursor-light.png"
            }
        }
        "opencode" => {
            if dark {
                "brands/opencode.png"
            } else {
                "brands/opencode-light.png"
            }
        }
        "pi" => {
            if dark {
                "brands/pi.png"
            } else {
                "brands/pi-light.png"
            }
        }
        "grok" => {
            if dark {
                "brands/grok.png"
            } else {
                "brands/grok-light.png"
            }
        }
        "kimi" | "moonshot" => {
            if dark {
                "brands/kimi.png"
            } else {
                "brands/kimi-light.png"
            }
        }
        "gemini" => "brands/gemini.png",
        "antigravity" => "brands/antigravity.png",
        "qoder" => {
            if dark {
                "brands/qoder.png"
            } else {
                "brands/qoder-light.png"
            }
        }
        "openclaw" => "brands/openclaw.png",
        "ohmypi" | "omp" => "brands/omp.png",
        "workbuddy" => "brands/workbuddy.png",
        "zcode" | "zai" => "brands/zcode.png",
        "deepseek" => "brands/deepseek.png",
        "hermes" | "fx" => {
            if dark {
                "brands/hermes.png"
            } else {
                "brands/hermes-light.png"
            }
        }
        "newapi" => "brands/newapi-logo.svg",
        "qwen" => "brands/qwen.svg",
        "glm" | "zhipu" => "brands/glm.svg",
        "minimax" => "brands/minimax.svg",
        "doubao" => "brands/doubao.svg",
        "custom" => "brands/custom.svg",
        _ => "brands/codex.png",
    }
}

/// Color-preserving brand icon (do not wrap in `Icon` / `text_color`).
pub fn brand_img(app_id: &str, dark: bool, size: impl Into<AbsoluteLength>) -> Img {
    img(brand_icon_path(app_id, dark))
        .size(size.into())
        .flex_shrink_0()
}

/// Brand icon sized for typical list / sidebar rows (18px).
pub fn brand_img_sm(app_id: &str, dark: bool) -> Img {
    brand_img(app_id, dark, px(18.))
}

/// Map CustomIcon brand variants to app ids used by [`brand_icon_path`].
pub fn custom_icon_brand_id(icon: CustomIcon) -> Option<&'static str> {
    match icon {
        CustomIcon::Claude => Some("claude"),
        CustomIcon::OpenAI => Some("codex"),
        CustomIcon::Grok => Some("grok"),
        CustomIcon::OpenCode => Some("opencode"),
        CustomIcon::Kimi => Some("kimi"),
        CustomIcon::Pi => Some("pi"),
        CustomIcon::Cursor => Some("cursor"),
        CustomIcon::ZCode => Some("zcode"),
        CustomIcon::WorkBuddy => Some("workbuddy"),
        CustomIcon::DeepSeek => Some("deepseek"),
        CustomIcon::Fx => Some("hermes"),
        CustomIcon::OhMyPi => Some("openclaw"),
        CustomIcon::Gemini => Some("gemini"),
        CustomIcon::Antigravity => Some("antigravity"),
        CustomIcon::Qoder => Some("qoder"),
        CustomIcon::Amp => None,
        _ => None,
    }
}

const AMP_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 21 21">
  <path fill="currentColor" d="m3.769 18.302 4.729-4.797 1.722 6.535 2.5-.684-2.491-9.489L.891 7.338.226 9.893l6.425 1.746-4.71 4.789 1.828 1.874Z"/>
  <path fill="currentColor" d="m17.407 12.741 2.501-.683-2.491-9.489L8.079.04l-.665 2.555 7.885 2.142 2.108 8.004Z"/>
  <path fill="currentColor" d="m13.818 16.388 2.501-.684-2.491-9.488L4.49 3.687l-.665 2.555 7.885 2.142 2.108 8.004Z"/>
</svg>"#;

const ACTIVITY_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
  <path d="M22 12h-4l-3 9L9 3l-3 9H2"/>
</svg>"#;

const KEY_ROUND_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
  <path d="M2.586 17.414A2 2 0 0 0 2 18.828V21a1 1 0 0 0 1 1h3a1 1 0 0 0 1-1v-1a1 1 0 0 1 1-1h1a1 1 0 0 0 1-1v-1a1 1 0 0 1 1-1h.172a2 2 0 0 0 1.414-.586l.814-.814a6.5 6.5 0 1 0-4-4z"/>
  <circle cx="16.5" cy="7.5" r=".5" fill="currentColor"/>
</svg>"#;

const DOWNLOAD_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
  <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
  <polyline points="7 10 12 15 17 10"/>
  <line x1="12" x2="12" y1="15" y2="3"/>
</svg>"#;

const ARROW_UP_DOWN_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
  <path d="m21 16-4 4-4-4"/>
  <path d="M17 20V4"/>
  <path d="m3 8 4-4 4 4"/>
  <path d="M7 4v16"/>
</svg>"#;

const HELP_CIRCLE_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
  <circle cx="12" cy="12" r="10"/>
  <path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/>
  <path d="M12 17h.01"/>
</svg>"#;

const ROTATE_CW_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
  <path d="M21 12a9 9 0 1 1-9-9c2.52 0 4.93 1 6.74 2.74L21 8"/>
  <path d="M21 3v5h-5"/>
</svg>"#;

/// AstrLink / Lucide RefreshCw (two arrows) — used on Gateway refresh.
const REFRESH_CW_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
  <path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8"/>
  <path d="M21 3v5h-5"/>
  <path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16"/>
  <path d="M8 16H3v5"/>
</svg>"#;

const ARROW_LEFT_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 56 56">
  <path fill="currentColor" d="M6.238 28c0 .586.258 1.125.727 1.57l15.562 15.54c.492.445.985.656 1.524.656c1.172 0 2.062-.844 2.062-2.016c0-.562-.21-1.125-.586-1.477l-5.226-5.343l-7.922-7.196l5.695.352H47.7c1.219 0 2.063-.867 2.063-2.086s-.844-2.086-2.063-2.086H18.074l-5.672.352l7.899-7.196l5.226-5.343c.399-.376.586-.915.586-1.477c0-1.172-.89-2.016-2.062-2.016c-.54 0-1.055.188-1.57.704L6.964 26.43c-.469.445-.727.984-.727 1.57"/>
</svg>"#;

const HISTORY_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
  <path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/>
  <path d="M3 3v5h5"/>
  <path d="M12 7v5l4 2"/>
</svg>"#;

const BOOK_OPEN_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
  <path d="M2 3h6a4 4 0 0 1 4 4v14a3 3 0 0 0-3-3H2z"/>
  <path d="M22 3h-6a4 4 0 0 0-4 4v14a3 3 0 0 1 3-3h7z"/>
</svg>"#;

/// AstrLink list action: SquarePen (edit).
const SQUARE_PEN_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
  <path d="M12 3H5a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/>
  <path d="M18.375 2.625a1 1 0 0 1 3 3l-9.013 9.014a2 2 0 0 1-.853.505l-2.873.84a.5.5 0 0 1-.62-.62l.84-2.873a2 2 0 0 1 .506-.852z"/>
</svg>"#;

/// AstrLink list action: Flask / FlaskConical (connectivity test).
const FLASK_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
  <path d="M14 2v6a2 2 0 0 0 .245.96l5.51 10.08A2 2 0 0 1 18 22H6a2 2 0 0 1-1.755-2.96l5.51-10.08A2 2 0 0 0 10 8V2"/>
  <path d="M6.453 15h11.094"/>
  <path d="M8.5 2h7"/>
</svg>"#;

/// AstrLink OrderedList drag handle.
const GRIP_VERTICAL_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
  <circle cx="9" cy="12" r="1"/>
  <circle cx="9" cy="5" r="1"/>
  <circle cx="9" cy="19" r="1"/>
  <circle cx="15" cy="12" r="1"/>
  <circle cx="15" cy="5" r="1"/>
  <circle cx="15" cy="19" r="1"/>
</svg>"#;

const MONITOR_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
  <rect width="20" height="14" x="2" y="3" rx="2"/>
  <line x1="8" x2="16" y1="21" y2="21"/>
  <line x1="12" x2="12" y1="17" y2="21"/>
</svg>"#;

const CHART_CURVE_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1000 200" preserveAspectRatio="none">
  <defs>
    <linearGradient id="claudeGrad" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%" stop-color="#D97757" stop-opacity="0.3" />
      <stop offset="100%" stop-color="#D97757" stop-opacity="0.02" />
    </linearGradient>
  </defs>
  <path d="M 0 5 C 10 5, 20 180, 36 194 C 52 198, 120 198, 1000 198 L 1000 200 L 0 200 Z" fill="url(#claudeGrad)" />
  <path d="M 0 5 C 10 5, 20 180, 36 194 C 52 198, 120 198, 1000 198" fill="none" stroke="#D97757" stroke-width="2.5" stroke-linecap="round" />
  <path d="M 0 195 C 15 195, 30 198, 60 198 L 1000 198" fill="none" stroke="#374151" stroke-width="2" stroke-linecap="round" />
  <line x1="0" y1="200" x2="1000" y2="200" stroke="#374151" stroke-width="2" />
</svg>"##;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CustomIcon {
    OpenAI,
    Claude,
    Grok,
    OpenCode,
    Kimi,
    Pi,
    Amp,
    Cursor,
    ZCode,
    WorkBuddy,
    DeepSeek,
    Fx,
    OhMyPi,
    Gemini,
    Antigravity,
    Qoder,
    RotateCw,
    /// AstrLink RefreshCw (two-arrow sync).
    RefreshCw,
    ArrowLeft,
    Monitor,
    History,
    BookOpen,
    ChartCurve,
    Activity,
    KeyRound,
    HelpCircle,
    Download,
    ArrowUpDown,
    /// AstrLink service-row edit (SquarePen).
    SquarePen,
    /// AstrLink service-row connectivity test (Flask).
    Flask,
    /// AstrLink OrderedList drag handle.
    GripVertical,
}

impl IconNamed for CustomIcon {
    fn path(self) -> SharedString {
        // Brand variants still expose IconNamed for Button/Select compatibility,
        // but UI should prefer [`brand_img`] so colors are preserved.
        match self {
            Self::OpenAI => "brands/codex.png",
            Self::Claude => "brands/claude-code.png",
            Self::Grok => "brands/grok.png",
            Self::OpenCode => "brands/opencode.png",
            Self::Kimi => "brands/kimi.png",
            Self::Pi => "brands/pi.png",
            Self::Amp => "icons/custom/amp.svg",
            Self::Cursor => "brands/cursor.png",
            Self::ZCode => "brands/zcode.png",
            Self::WorkBuddy => "brands/workbuddy.png",
            Self::DeepSeek => "brands/deepseek.png",
            Self::Fx => "brands/hermes.png",
            Self::OhMyPi => "brands/openclaw.png",
            Self::Gemini => "brands/gemini.png",
            Self::Antigravity => "brands/antigravity.png",
            Self::Qoder => "brands/qoder.png",
            Self::RotateCw => "icons/custom/rotate-cw.svg",
            Self::RefreshCw => "icons/custom/refresh-cw.svg",
            Self::ArrowLeft => "icons/custom/arrow-left.svg",
            Self::Monitor => "icons/custom/monitor.svg",
            Self::History => "icons/custom/history.svg",
            Self::BookOpen => "icons/custom/book-open.svg",
            Self::ChartCurve => "icons/custom/chart-curve.svg",
            Self::Activity => "icons/custom/activity.svg",
            Self::KeyRound => "icons/custom/key-round.svg",
            Self::HelpCircle => "icons/custom/help-circle.svg",
            Self::Download => "icons/custom/download.svg",
            Self::ArrowUpDown => "icons/custom/arrow-up-down.svg",
            Self::SquarePen => "icons/custom/square-pen.svg",
            Self::Flask => "icons/custom/flask.svg",
            Self::GripVertical => "icons/custom/grip-vertical.svg",
        }
        .into()
    }
}

impl AssetSource for AppAssets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        if let Some(bytes) = lookup_brand(path) {
            return Ok(Some(Cow::Borrowed(bytes)));
        }
        match path {
            "icons/custom/amp.svg" => Ok(Some(Cow::Borrowed(AMP_SVG.as_bytes()))),
            "icons/custom/rotate-cw.svg" => Ok(Some(Cow::Borrowed(ROTATE_CW_SVG.as_bytes()))),
            "icons/custom/refresh-cw.svg" => Ok(Some(Cow::Borrowed(REFRESH_CW_SVG.as_bytes()))),
            "icons/custom/arrow-left.svg" => Ok(Some(Cow::Borrowed(ARROW_LEFT_SVG.as_bytes()))),
            "icons/custom/monitor.svg" => Ok(Some(Cow::Borrowed(MONITOR_SVG.as_bytes()))),
            "icons/custom/history.svg" => Ok(Some(Cow::Borrowed(HISTORY_SVG.as_bytes()))),
            "icons/custom/book-open.svg" => Ok(Some(Cow::Borrowed(BOOK_OPEN_SVG.as_bytes()))),
            "icons/custom/chart-curve.svg" => Ok(Some(Cow::Borrowed(CHART_CURVE_SVG.as_bytes()))),
            "icons/custom/activity.svg" => Ok(Some(Cow::Borrowed(ACTIVITY_SVG.as_bytes()))),
            "icons/custom/key-round.svg" => Ok(Some(Cow::Borrowed(KEY_ROUND_SVG.as_bytes()))),
            "icons/custom/help-circle.svg" => Ok(Some(Cow::Borrowed(HELP_CIRCLE_SVG.as_bytes()))),
            "icons/custom/download.svg" => Ok(Some(Cow::Borrowed(DOWNLOAD_SVG.as_bytes()))),
            "icons/custom/arrow-up-down.svg" => {
                Ok(Some(Cow::Borrowed(ARROW_UP_DOWN_SVG.as_bytes())))
            }
            "icons/custom/square-pen.svg" => Ok(Some(Cow::Borrowed(SQUARE_PEN_SVG.as_bytes()))),
            "icons/custom/flask.svg" => Ok(Some(Cow::Borrowed(FLASK_SVG.as_bytes()))),
            "icons/custom/grip-vertical.svg" => {
                Ok(Some(Cow::Borrowed(GRIP_VERTICAL_SVG.as_bytes())))
            }
            "brands/newapi-logo.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/brands/newapi-logo.svg"
            )))),
            "brands/qwen.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/brands/qwen.svg"
            )))),
            "brands/glm.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/brands/glm.svg"
            )))),
            "brands/minimax.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/brands/minimax.svg"
            )))),
            "brands/doubao.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/brands/doubao.svg"
            )))),
            "brands/custom.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/brands/custom.svg"
            )))),
            _ => gpui_component_assets::Assets.load(path),
        }
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
        let mut list = gpui_component_assets::Assets.list(path)?;
        if path.is_empty() || path == "icons" || path == "icons/custom" || path == "brands" {
            list.push("brands/claude-code.png".into());
            list.push("brands/codex.png".into());
            list.push("brands/cursor.png".into());
            list.push("brands/opencode.png".into());
            list.push("brands/pi.png".into());
            list.push("brands/grok.png".into());
            list.push("brands/kimi.png".into());
            list.push("brands/gemini.png".into());
            list.push("brands/antigravity.png".into());
            list.push("brands/qoder.png".into());
            list.push("brands/openclaw.png".into());
            list.push("brands/workbuddy.png".into());
            list.push("brands/zcode.png".into());
            list.push("icons/custom/amp.svg".into());
            list.push("icons/custom/rotate-cw.svg".into());
            list.push("icons/custom/arrow-left.svg".into());
            list.push("icons/custom/monitor.svg".into());
            list.push("icons/custom/history.svg".into());
            list.push("icons/custom/book-open.svg".into());
            list.push("icons/custom/chart-curve.svg".into());
            list.push("icons/custom/activity.svg".into());
            list.push("icons/custom/square-pen.svg".into());
            list.push("icons/custom/flask.svg".into());
            list.push("icons/custom/grip-vertical.svg".into());
        }
        Ok(list)
    }
}
