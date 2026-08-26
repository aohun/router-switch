use gpui::{
    hsla, px, rgb, rgba, App, Div, FontWeight, Hsla, IntoElement, ParentElement, Styled, Window,
};
use gpui_component::theme::{Theme, ThemeMode};
use store::ThemePreference;

/// Fixed status colors for status pills.
#[allow(dead_code)]
pub struct StatusColors;

#[allow(dead_code)]
impl StatusColors {
    pub const BLUE_100: u32 = 0xDBEAFE;
    pub const BLUE_500: u32 = 0x3B82F6;
    pub const BLUE_800: u32 = 0x1E40AF;
    pub const GREEN_100: u32 = 0xDCFCE7;
    pub const GREEN_500: u32 = 0x22C55E;
    pub const GREEN_700: u32 = 0x15803D;
    pub const AMBER_100: u32 = 0xFEF3C7;
    pub const AMBER_500: u32 = 0xF59E0B;
    pub const AMBER_700: u32 = 0xB45309;
    pub const RED_100: u32 = 0xFEE2E2;
    pub const RED_500: u32 = 0xEF4444;
    pub const RED_700: u32 = 0xB91C1C;
    pub const GRAY_500: u32 = 0x6B7280;
}

#[allow(dead_code)]
pub fn label(preference: ThemePreference) -> &'static str {
    match preference {
        ThemePreference::System => "跟随系统",
        ThemePreference::Light => "浅色",
        ThemePreference::Dark => "深色",
    }
}

#[allow(dead_code)]
pub fn next(preference: ThemePreference) -> ThemePreference {
    match preference {
        ThemePreference::System => ThemePreference::Light,
        ThemePreference::Light => ThemePreference::Dark,
        ThemePreference::Dark => ThemePreference::System,
    }
}

pub fn apply_theme(preference: ThemePreference, window: Option<&mut Window>, cx: &mut App) {
    match preference {
        ThemePreference::System => Theme::sync_system_appearance(window, cx),
        ThemePreference::Light => Theme::change(ThemeMode::Light, window, cx),
        ThemePreference::Dark => Theme::change(ThemeMode::Dark, window, cx),
    }
    apply_palette(cx);
}

fn h(hex: u32) -> Hsla {
    rgb(hex).into()
}

fn ha(hex: u32, alpha: f32) -> Hsla {
    let mut c: Hsla = rgb(hex).into();
    c.a = alpha;
    c
}

/// Override gpui-component's default theme with custom palette.
pub fn apply_palette(cx: &mut App) {
    let dark = Theme::global(cx).is_dark();
    let theme = Theme::global_mut(cx);

    if !dark {
        theme.background = h(0xFFFFFF);
        theme.foreground = h(0x0A0A0A);
        theme.popover = h(0xFFFFFF);
        theme.popover_foreground = h(0x0A0A0A);
        theme.primary = h(0x171717);
        theme.primary_hover = h(0x2E2E2E);
        theme.primary_active = h(0x000000);
        theme.primary_foreground = h(0xFAFAFA);
        theme.secondary = h(0xF5F5F5);
        theme.secondary_hover = h(0xEBEBEB);
        theme.secondary_active = h(0xE0E0E0);
        theme.secondary_foreground = h(0x171717);
        theme.muted = h(0xF5F5F5);
        theme.muted_foreground = h(0x737373);
        theme.accent = h(0xF5F5F5);
        theme.accent_foreground = h(0x171717);
        theme.border = h(0xE5E5E5);
        theme.input = h(0xE5E5E5);
        theme.ring = h(0xA1A1A1);
        theme.danger = h(0xE7000B);
        theme.danger_foreground = h(0xFFFFFF);
        // Translucent sidebar over the blurred window background (macOS vibrancy).
        theme.sidebar = rgba(0xFFFFFF8C).into();
        theme.sidebar_foreground = h(0x0A0A0A);
        theme.sidebar_accent = hsla(0., 0., 0., 0.06);
        theme.sidebar_accent_foreground = h(0x171717);
        theme.sidebar_border = ha(0xE5E5E5, 0.5);
        theme.sidebar_primary = h(0x171717);
        theme.sidebar_primary_foreground = h(0xFAFAFA);
        theme.title_bar = hsla(0., 0., 0., 0.);
        theme.title_bar_border = hsla(0., 0., 0., 0.);
        theme.tab_bar = h(0xEEEEEE);
        theme.tab = hsla(0., 0., 0., 0.);
        theme.tab_active = h(0xFFFFFF);
        theme.tab_foreground = ha(0x0A0A0A, 0.6);
        theme.tab_active_foreground = h(0x0A0A0A);
        theme.progress_bar = h(StatusColors::BLUE_500);
        theme.scrollbar_thumb = hsla(0., 0., 0., 0.2);
    } else {
        theme.background = h(0x0A0A0A);
        theme.foreground = h(0xFAFAFA);
        theme.popover = h(0x171717);
        theme.popover_foreground = h(0xFAFAFA);
        theme.primary = h(0xE5E5E5);
        theme.primary_hover = h(0xD4D4D4);
        theme.primary_active = h(0xFFFFFF);
        theme.primary_foreground = h(0x171717);
        theme.secondary = h(0x262626);
        theme.secondary_hover = h(0x303030);
        theme.secondary_active = h(0x3A3A3A);
        theme.secondary_foreground = h(0xFAFAFA);
        theme.muted = h(0x262626);
        theme.muted_foreground = h(0xA1A1A1);
        theme.accent = h(0x262626);
        theme.accent_foreground = h(0xFAFAFA);
        theme.border = hsla(0., 0., 1., 0.10);
        theme.input = hsla(0., 0., 1., 0.15);
        theme.ring = h(0x737373);
        theme.danger = h(0xFF6467);
        theme.danger_foreground = h(0xFFFFFF);
        theme.sidebar = rgba(0x1717178C).into();
        theme.sidebar_foreground = h(0xFAFAFA);
        theme.sidebar_accent = hsla(0., 0., 1., 0.10);
        theme.sidebar_accent_foreground = h(0xFAFAFA);
        theme.sidebar_border = hsla(0., 0., 1., 0.08);
        theme.sidebar_primary = h(0x4436C9);
        theme.sidebar_primary_foreground = h(0xFAFAFA);
        theme.title_bar = hsla(0., 0., 0., 0.);
        theme.title_bar_border = hsla(0., 0., 0., 0.);
        theme.tab_bar = h(0x262626);
        theme.tab = hsla(0., 0., 0., 0.);
        theme.tab_active = h(0x0A0A0A);
        theme.tab_foreground = ha(0xFAFAFA, 0.6);
        theme.tab_active_foreground = h(0xFAFAFA);
        theme.progress_bar = h(StatusColors::BLUE_500);
        theme.scrollbar_thumb = hsla(0., 0., 1., 0.2);
    }
    theme.radius = px(8.);
    theme.radius_lg = px(14.);
}

/// The main content "inset card" background (sidebar-inset token).
pub fn inset_bg(dark: bool) -> Hsla {
    if dark {
        ha(0x0A0A0A, 0.80)
    } else {
        ha(0xFFFFFF, 0.88)
    }
}

/// Router Switch tile container style.
pub fn tile(cx: &App) -> Div {
    let dark = Theme::global(cx).is_dark();
    gpui::div()
        .rounded(px(16.))
        .border_1()
        .border_color(if dark {
            hsla(0., 0., 1., 0.10)
        } else {
            hsla(0., 0., 0., 0.07)
        })
        .bg(if dark {
            rgba(0x212121F2)
        } else {
            rgba(0xFFFFFFF5)
        })
        .p(px(16.))
        .overflow_hidden()
}

/// Router Switch tile uppercase label.
pub fn tile_label(text: impl Into<gpui::SharedString>, cx: &App) -> impl IntoElement {
    let muted = Theme::global(cx).muted_foreground;
    let s = text.into();
    gpui::div()
        .text_size(px(11.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(muted)
        .child(s.to_string().to_uppercase())
}
