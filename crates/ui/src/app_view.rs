use domain::{
    check_app_update, extract_claude_base_url, extract_claude_model, extract_codex_base_url,
    extract_codex_model, extract_cursor_base_url, extract_cursor_model, extract_grok_base_url,
    extract_grok_model, extract_opencode_base_url, extract_opencode_model, extract_zcode_base_url,
    extract_zcode_model, parse_clipboard_provider_info, sample_app_release, AppKind, AppRelease,
    ClaudeForm, ClaudeKind, ClaudeModelMapping, ClipboardProviderInfo, CodexForm, CodexKind,
    CodexModelMapping, CursorForm, CursorKind, CursorModelMapping, GrokForm, GrokKind,
    GrokModelMapping, OpenCodeForm, OpenCodeKind, OpenCodeModelMapping, PiForm, PiKind,
    PiModelMapping, Provider, ProviderForm, ProviderSettings, ToolEnvironmentStatus, WorkBuddyForm,
    WorkBuddyKind, ZCodeForm, ZCodeKind, ZCodeModelMapping, CLAUDE_PRESETS, CURSOR_PRESETS,
    DEFAULT_CLAUDE_MODEL, DEFAULT_CODEX_MODEL, DEFAULT_CURSOR_MODEL, DEFAULT_GROK_MODEL,
    DEFAULT_OPENCODE_MODEL, DEFAULT_PI_MODEL, DEFAULT_WORKBUDDY_MODEL, DEFAULT_WORKBUDDY_VENDOR,
    DEFAULT_ZCODE_MODEL, DEFAULT_ZCODE_PROVIDER_KIND, GROK_PRESETS, OPENCODE_PRESETS, PI_PRESETS,
    RESPONSES_PRESETS, WORKBUDDY_PRESETS, ZCODE_PRESETS,
};
use gpui::{
    div, prelude::FluentBuilder, px, rgb, rgba, App, AppContext, Context, Entity, FontWeight, Hsla,
    InteractiveElement, IntoElement, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, Subscription, Window, WindowControlArea,
};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputState},
    notification::Notification,
    scroll::ScrollableElement,
    select::{Select, SelectEvent, SelectItem, SelectState},
    tag::Tag,
    v_flex, ActiveTheme, Disableable as _, Icon, IconName, Selectable as _, Sizable as _,
    WindowExt,
};
use rust_i18n::t;
use session::Workspace;
use store::{AppLanguage, ThemePreference};

use crate::assets::CustomIcon;
use crate::theme;
use crate::update_dialog::open_app_update_dialog;
pub use crate::usage_service::*;

pub const CHROME_HEIGHT: f32 = 46.;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DragAppId(pub String);

pub struct DragGhostView {
    pub label: SharedString,
}

impl Render for DragGhostView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .px(px(12.))
            .py(px(6.))
            .rounded(px(8.))
            .bg(cx.theme().primary)
            .text_color(cx.theme().primary_foreground)
            .text_size(px(13.))
            .font_weight(FontWeight::SEMIBOLD)
            .shadow_md()
            .opacity(0.9)
            .child(self.label.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresetSelectItem {
    pub id: String,
    pub name: String,
    pub website_url: String,
    pub base_url: String,
    pub model: String,
    pub is_official: bool,
    pub provider_label: String,
    pub modality_text: bool,
    pub modality_image: bool,
}

impl SelectItem for PresetSelectItem {
    type Value = String;

    fn title(&self) -> SharedString {
        self.name.clone().into()
    }

    fn value(&self) -> &Self::Value {
        &self.id
    }

    fn render(&self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let is_official = self.is_official;
        h_flex()
            .items_center()
            .w_full()
            .gap(px(8.))
            .child(if is_official {
                IconName::Bot
            } else {
                IconName::SquareTerminal
            })
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(cx.theme().foreground)
                    .child(self.name.clone()),
            )
    }

    fn matches(&self, query: &str) -> bool {
        let q = query.to_lowercase();
        self.name.to_lowercase().contains(&q)
            || self.base_url.to_lowercase().contains(&q)
            || self.id.to_lowercase().contains(&q)
    }
}

pub fn presets_for_app(app: AppKind) -> Vec<PresetSelectItem> {
    match app {
        AppKind::Codex => RESPONSES_PRESETS
            .iter()
            .map(|p| PresetSelectItem {
                id: p.id.to_string(),
                name: p.name.to_string(),
                website_url: p.website_url.to_string(),
                base_url: p.base_url.to_string(),
                model: p.model.to_string(),
                is_official: p.kind.is_official(),
                provider_label: p.provider_label.to_string(),
                modality_text: true,
                modality_image: true,
            })
            .collect(),
        AppKind::Claude => CLAUDE_PRESETS
            .iter()
            .map(|p| PresetSelectItem {
                id: p.id.to_string(),
                name: p.name.to_string(),
                website_url: p.website_url.to_string(),
                base_url: p.base_url.to_string(),
                model: p.model.to_string(),
                is_official: p.kind.is_official(),
                provider_label: p.provider_label.to_string(),
                modality_text: true,
                modality_image: true,
            })
            .collect(),
        AppKind::Grok => GROK_PRESETS
            .iter()
            .map(|p| PresetSelectItem {
                id: p.id.to_string(),
                name: p.name.to_string(),
                website_url: p.website_url.to_string(),
                base_url: p.base_url.to_string(),
                model: p.model.to_string(),
                is_official: p.kind.is_official(),
                provider_label: p.provider_label.to_string(),
                modality_text: true,
                modality_image: true,
            })
            .collect(),
        AppKind::OpenCode => OPENCODE_PRESETS
            .iter()
            .map(|p| PresetSelectItem {
                id: p.id.to_string(),
                name: p.name.to_string(),
                website_url: p.website_url.to_string(),
                base_url: p.base_url.to_string(),
                model: p.model.to_string(),
                is_official: p.kind.is_official(),
                provider_label: p.provider_label.to_string(),
                modality_text: true,
                modality_image: true,
            })
            .collect(),
        AppKind::Pi => PI_PRESETS
            .iter()
            .map(|p| PresetSelectItem {
                id: p.id.to_string(),
                name: p.name.to_string(),
                website_url: p.website_url.to_string(),
                base_url: p.base_url.to_string(),
                model: p.model.to_string(),
                is_official: p.kind.is_official(),
                provider_label: p.provider_label.to_string(),
                modality_text: true,
                modality_image: true,
            })
            .collect(),
        AppKind::Cursor => CURSOR_PRESETS
            .iter()
            .map(|p| PresetSelectItem {
                id: p.id.to_string(),
                name: p.name.to_string(),
                website_url: p.website_url.to_string(),
                base_url: p.base_url.to_string(),
                model: p.model.to_string(),
                is_official: p.kind.is_official(),
                provider_label: p.provider_label.to_string(),
                modality_text: true,
                modality_image: true,
            })
            .collect(),
        AppKind::ZCode => ZCODE_PRESETS
            .iter()
            .map(|p| PresetSelectItem {
                id: p.id.to_string(),
                name: p.name.to_string(),
                website_url: p.website_url.to_string(),
                base_url: p.base_url.to_string(),
                model: p.model.to_string(),
                is_official: p.kind.is_official(),
                provider_label: p.provider_label.to_string(),
                modality_text: p.modality_text,
                modality_image: p.modality_image,
            })
            .collect(),
        AppKind::WorkBuddy => WORKBUDDY_PRESETS
            .iter()
            .map(|p| PresetSelectItem {
                id: p.id.to_string(),
                name: p.name.to_string(),
                website_url: p.website_url.to_string(),
                base_url: p.base_url.to_string(),
                model: p.model_id.to_string(),
                is_official: p.kind.is_official(),
                provider_label: p.provider_label.to_string(),
                modality_text: true,
                modality_image: p.supports_images,
            })
            .collect(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelSelectItem {
    pub name: String,
}

impl SelectItem for ModelSelectItem {
    type Value = String;

    fn title(&self) -> SharedString {
        self.name.clone().into()
    }

    fn value(&self) -> &Self::Value {
        &self.name
    }

    fn render(&self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .text_size(px(12.))
            .text_color(cx.theme().foreground)
            .child(self.name.clone())
    }

    fn matches(&self, query: &str) -> bool {
        self.name.to_lowercase().contains(&query.to_lowercase())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReasoningOptionItem {
    pub label: String,
    pub value: Option<String>,
}

impl SelectItem for ReasoningOptionItem {
    type Value = Option<String>;

    fn title(&self) -> SharedString {
        self.label.clone().into()
    }

    fn value(&self) -> &Self::Value {
        &self.value
    }

    fn render(&self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .text_size(px(12.))
            .text_color(cx.theme().foreground)
            .child(self.label.clone())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkBuddyReasoningEffortItem {
    pub label: SharedString,
    pub value: String,
}

impl SelectItem for WorkBuddyReasoningEffortItem {
    type Value = String;

    fn title(&self) -> SharedString {
        self.label.clone()
    }

    fn value(&self) -> &Self::Value {
        &self.value
    }

    fn render(&self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .text_size(px(13.))
            .text_color(cx.theme().foreground)
            .child(self.label.clone())
    }
}

pub struct CatalogRowDraft {
    pub display_name: Entity<InputState>,
    pub model: Entity<InputState>,
    pub context_window: Entity<InputState>,
    pub reasoning_effort: Entity<SelectState<Vec<ReasoningOptionItem>>>,
    pub model_select: Option<Entity<SelectState<Vec<ModelSelectItem>>>>,
    pub _model_select_sub: Option<Subscription>,
}

impl CatalogRowDraft {
    pub fn new(
        display_name_val: &str,
        model_val: &str,
        context_window_val: Option<u64>,
        reasoning_effort_val: Option<&str>,
        fetched_models: &[String],
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        let display_name = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("例如: DeepSeek V4 Flash")
                .default_value(display_name_val.to_string())
        });
        let model = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("例如: deepseek-v4-flash")
                .default_value(model_val.to_string())
        });
        let context_str = context_window_val
            .map(|n| n.to_string())
            .unwrap_or_default();
        let context_window = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("例如: 128000")
                .default_value(context_str)
        });

        let options = vec![
            ReasoningOptionItem {
                label: "未设置".into(),
                value: None,
            },
            ReasoningOptionItem {
                label: "low".into(),
                value: Some("low".into()),
            },
            ReasoningOptionItem {
                label: "medium".into(),
                value: Some("medium".into()),
            },
            ReasoningOptionItem {
                label: "high".into(),
                value: Some("high".into()),
            },
        ];
        let selected_idx = match reasoning_effort_val {
            Some("low") => Some(1),
            Some("medium") => Some(2),
            Some("high") => Some(3),
            _ => Some(0),
        };
        let reasoning_effort = cx.new(|cx| {
            let index_path = selected_idx.map(|i| gpui_component::IndexPath::default().row(i));
            SelectState::new(options, index_path, window, cx)
        });

        let (model_select, _model_select_sub) = if !fetched_models.is_empty() {
            let items: Vec<ModelSelectItem> = fetched_models
                .iter()
                .map(|m| ModelSelectItem { name: m.clone() })
                .collect();
            let selected_model_idx = fetched_models
                .iter()
                .position(|m| m == model_val)
                .map(|i| gpui_component::IndexPath::default().row(i));
            let select = cx
                .new(|cx| SelectState::new(items, selected_model_idx, window, cx).searchable(true));
            let model_state = model.clone();
            let display_name_state = display_name.clone();
            let sub = window.subscribe(
                &select,
                cx,
                move |_, event: &SelectEvent<Vec<ModelSelectItem>>, window, cx| {
                    if let SelectEvent::Confirm(Some(m)) = event {
                        let val = m.clone();
                        model_state
                            .update(cx, |input, cx| input.set_value(val.clone(), window, cx));
                        display_name_state.update(cx, |input, cx| {
                            let curr = input.value().to_string();
                            if curr.trim().is_empty() {
                                input.set_value(val, window, cx);
                            }
                        });
                    }
                },
            );
            (Some(select), Some(sub))
        } else {
            (None, None)
        };

        Self {
            display_name,
            model,
            context_window,
            reasoning_effort,
            model_select,
            _model_select_sub,
        }
    }

    pub fn set_fetched_models(
        &mut self,
        fetched_models: &[String],
        window: &mut Window,
        cx: &mut App,
    ) {
        if fetched_models.is_empty() {
            self.model_select = None;
            self._model_select_sub = None;
            return;
        }

        let current_val = self.model.read(cx).value().to_string();
        let items: Vec<ModelSelectItem> = fetched_models
            .iter()
            .map(|m| ModelSelectItem { name: m.clone() })
            .collect();
        let selected_model_idx = fetched_models
            .iter()
            .position(|m| m == &current_val)
            .map(|i| gpui_component::IndexPath::default().row(i));
        let select =
            cx.new(|cx| SelectState::new(items, selected_model_idx, window, cx).searchable(true));
        let model_state = self.model.clone();
        let display_name_state = self.display_name.clone();
        let sub = window.subscribe(
            &select,
            cx,
            move |_, event: &SelectEvent<Vec<ModelSelectItem>>, window, cx| {
                if let SelectEvent::Confirm(Some(m)) = event {
                    let val = m.clone();
                    model_state.update(cx, |input, cx| input.set_value(val.clone(), window, cx));
                    display_name_state.update(cx, |input, cx| {
                        let curr = input.value().to_string();
                        if curr.trim().is_empty() {
                            input.set_value(val, window, cx);
                        }
                    });
                }
            },
        );
        self.model_select = Some(select);
        self._model_select_sub = Some(sub);
    }

    pub fn to_codex_mapping(&self, cx: &App) -> Option<CodexModelMapping> {
        let model_val = self.model.read(cx).value().to_string();
        let model_trimmed = model_val.trim();
        if model_trimmed.is_empty() {
            return None;
        }
        let display_name_val = self.display_name.read(cx).value().to_string();
        let context_str = self.context_window.read(cx).value().to_string();
        let context_window = context_str.trim().parse::<u64>().ok();
        let reasoning_effort = self
            .reasoning_effort
            .read(cx)
            .selected_value()
            .cloned()
            .flatten();

        Some(CodexModelMapping {
            display_name: if display_name_val.trim().is_empty() {
                model_trimmed.to_string()
            } else {
                display_name_val.trim().to_string()
            },
            model: model_trimmed.to_string(),
            context_window,
            reasoning_effort,
        })
    }

    pub fn to_claude_mapping(&self, cx: &App) -> Option<ClaudeModelMapping> {
        let model_val = self.model.read(cx).value().to_string();
        let model_trimmed = model_val.trim();
        if model_trimmed.is_empty() {
            return None;
        }
        let display_name_val = self.display_name.read(cx).value().to_string();
        let context_str = self.context_window.read(cx).value().to_string();
        let context_window = context_str.trim().parse::<u64>().ok();
        let reasoning_effort = self
            .reasoning_effort
            .read(cx)
            .selected_value()
            .cloned()
            .flatten();

        Some(ClaudeModelMapping {
            display_name: if display_name_val.trim().is_empty() {
                model_trimmed.to_string()
            } else {
                display_name_val.trim().to_string()
            },
            model: model_trimmed.to_string(),
            context_window,
            reasoning_effort,
        })
    }

    pub fn to_grok_mapping(&self, cx: &App) -> Option<GrokModelMapping> {
        let model_val = self.model.read(cx).value().to_string();
        let model_trimmed = model_val.trim();
        if model_trimmed.is_empty() {
            return None;
        }
        let display_name_val = self.display_name.read(cx).value().to_string();
        let context_str = self.context_window.read(cx).value().to_string();
        let context_window = context_str.trim().parse::<u64>().ok();
        let reasoning_effort = self
            .reasoning_effort
            .read(cx)
            .selected_value()
            .cloned()
            .flatten();

        Some(GrokModelMapping {
            display_name: if display_name_val.trim().is_empty() {
                model_trimmed.to_string()
            } else {
                display_name_val.trim().to_string()
            },
            model: model_trimmed.to_string(),
            context_window,
            reasoning_effort,
        })
    }

    pub fn to_opencode_mapping(&self, cx: &App) -> Option<OpenCodeModelMapping> {
        let model_val = self.model.read(cx).value().to_string();
        let model_trimmed = model_val.trim();
        if model_trimmed.is_empty() {
            return None;
        }
        let display_name_val = self.display_name.read(cx).value().to_string();
        let context_str = self.context_window.read(cx).value().to_string();
        let context_window = context_str.trim().parse::<u64>().ok();

        Some(OpenCodeModelMapping {
            display_name: if display_name_val.trim().is_empty() {
                model_trimmed.to_string()
            } else {
                display_name_val.trim().to_string()
            },
            model_id: model_trimmed.to_string(),
            context_limit: context_window,
            output_limit: None,
        })
    }

    pub fn to_pi_mapping(&self, cx: &App) -> Option<PiModelMapping> {
        let model_val = self.model.read(cx).value().to_string();
        let model_trimmed = model_val.trim();
        if model_trimmed.is_empty() {
            return None;
        }
        let display_name_val = self.display_name.read(cx).value().to_string();
        let context_str = self.context_window.read(cx).value().to_string();
        let context_window = context_str.trim().parse::<u64>().ok();

        Some(PiModelMapping {
            display_name: if display_name_val.trim().is_empty() {
                model_trimmed.to_string()
            } else {
                display_name_val.trim().to_string()
            },
            model_id: model_trimmed.to_string(),
            context_window,
        })
    }

    pub fn to_cursor_mapping(&self, cx: &App) -> Option<CursorModelMapping> {
        let model_val = self.model.read(cx).value().to_string();
        let model_trimmed = model_val.trim();
        if model_trimmed.is_empty() {
            return None;
        }
        let display_name_val = self.display_name.read(cx).value().to_string();
        let context_str = self.context_window.read(cx).value().to_string();
        let context_window = context_str.trim().parse::<u64>().ok();
        let reasoning_effort = self
            .reasoning_effort
            .read(cx)
            .selected_value()
            .cloned()
            .flatten();

        Some(CursorModelMapping {
            display_name: if display_name_val.trim().is_empty() {
                model_trimmed.to_string()
            } else {
                display_name_val.trim().to_string()
            },
            model: model_trimmed.to_string(),
            context_window,
            reasoning_effort,
        })
    }

    pub fn to_zcode_mapping(&self, cx: &App) -> Option<ZCodeModelMapping> {
        let model_val = self.model.read(cx).value().to_string();
        let model_trimmed = model_val.trim();
        if model_trimmed.is_empty() {
            return None;
        }
        let display_name_val = self.display_name.read(cx).value().to_string();
        let context_str = self.context_window.read(cx).value().to_string();
        let context_limit = context_str.trim().parse::<u64>().ok();

        Some(ZCodeModelMapping {
            display_name: if display_name_val.trim().is_empty() {
                model_trimmed.to_string()
            } else {
                display_name_val.trim().to_string()
            },
            model_id: model_trimmed.to_string(),
            context_limit,
            output_limit: None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    Dashboard,
    Codex,
    Claude,
    Grok,
    OpenCode,
    Pi,
    Cursor,
    ZCode,
    WorkBuddy,
    Notifications,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsTab {
    #[default]
    General,
    Advanced,
    About,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageWindowSelectItem {
    pub choice: UsageWindowChoice,
    pub label: String,
}

impl SelectItem for UsageWindowSelectItem {
    type Value = UsageWindowChoice;
    fn title(&self) -> SharedString {
        self.label.clone().into()
    }
    fn value(&self) -> &Self::Value {
        &self.choice
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageRefreshSelectItem {
    pub interval: UsageRefreshInterval,
    pub label: String,
}

impl SelectItem for UsageRefreshSelectItem {
    type Value = UsageRefreshInterval;
    fn title(&self) -> SharedString {
        self.label.clone().into()
    }
    fn value(&self) -> &Self::Value {
        &self.interval
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogLevelSelectItem {
    pub level: store::LogLevel,
    pub label: String,
}

impl SelectItem for LogLevelSelectItem {
    type Value = store::LogLevel;
    fn title(&self) -> SharedString {
        self.label.clone().into()
    }
    fn value(&self) -> &Self::Value {
        &self.level
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogRetentionSelectItem {
    pub days: u32,
    pub label: String,
}

impl SelectItem for LogRetentionSelectItem {
    type Value = u32;
    fn title(&self) -> SharedString {
        self.label.clone().into()
    }
    fn value(&self) -> &Self::Value {
        &self.days
    }
}

fn log_level_items() -> Vec<LogLevelSelectItem> {
    vec![
        LogLevelSelectItem {
            level: store::LogLevel::Error,
            label: t!("advanced.log_config.levels.error").to_string(),
        },
        LogLevelSelectItem {
            level: store::LogLevel::Warn,
            label: t!("advanced.log_config.levels.warn").to_string(),
        },
        LogLevelSelectItem {
            level: store::LogLevel::Info,
            label: t!("advanced.log_config.levels.info").to_string(),
        },
        LogLevelSelectItem {
            level: store::LogLevel::Debug,
            label: t!("advanced.log_config.levels.debug").to_string(),
        },
        LogLevelSelectItem {
            level: store::LogLevel::Trace,
            label: t!("advanced.log_config.levels.trace").to_string(),
        },
    ]
}

fn log_retention_items() -> Vec<LogRetentionSelectItem> {
    vec![
        LogRetentionSelectItem {
            days: 3,
            label: t!("advanced.log_config.retention_days_3").to_string(),
        },
        LogRetentionSelectItem {
            days: 7,
            label: t!("advanced.log_config.retention_days_7").to_string(),
        },
        LogRetentionSelectItem {
            days: 14,
            label: t!("advanced.log_config.retention_days_14").to_string(),
        },
        LogRetentionSelectItem {
            days: 30,
            label: t!("advanced.log_config.retention_days_30").to_string(),
        },
        LogRetentionSelectItem {
            days: 0,
            label: t!("advanced.log_config.retention_days_forever").to_string(),
        },
    ]
}

fn log_level_to_row(level: store::LogLevel) -> usize {
    match level {
        store::LogLevel::Error => 0,
        store::LogLevel::Warn => 1,
        store::LogLevel::Info => 2,
        store::LogLevel::Debug => 3,
        store::LogLevel::Trace => 4,
    }
}

fn log_retention_to_row(days: u32) -> usize {
    match days {
        3 => 0,
        7 => 1,
        14 => 2,
        30 => 3,
        0 => 4,
        _ => 1,
    }
}

pub struct RouterApp {
    workspace: Workspace,
    providers: Vec<Provider>,
    route: Route,
    previous_route: Route,
    sidebar_open: bool,
    theme: ThemePreference,
    language: AppLanguage,
    main_apps: Vec<String>,
    launch_on_startup: bool,
    minimize_to_tray: bool,
    settings_tab: SettingsTab,
    dashboard_app_filter: Option<AppKind>,
    dashboard_data: Option<DashboardUsageData>,
    is_loading_dashboard: bool,
    usage_breakdown_tab: UsageBreakdownTab,
    usage_window: UsageWindowChoice,
    usage_metric: UsageMetric,
    usage_refresh_interval: UsageRefreshInterval,
    usage_window_select: Entity<SelectState<Vec<UsageWindowSelectItem>>>,
    usage_refresh_select: Entity<SelectState<Vec<UsageRefreshSelectItem>>>,
    _usage_window_sub: Option<Subscription>,
    _usage_refresh_sub: Option<Subscription>,
    log_config: store::LogConfig,
    log_level_select: Entity<SelectState<Vec<LogLevelSelectItem>>>,
    log_retention_select: Entity<SelectState<Vec<LogRetentionSelectItem>>>,
    _log_level_sub: Option<Subscription>,
    _log_retention_sub: Option<Subscription>,
    search_input: Entity<InputState>,
    settings_search_input: Entity<InputState>,
    last_error: Option<SharedString>,
    form: Option<FormDraft>,
    logs: Vec<String>,
    env_tools: Vec<ToolEnvironmentStatus>,
    is_inspecting_env: bool,
    auto_check_update: bool,
    skipped_update_version: Option<String>,
    is_checking_update: bool,
    testing_provider_ids: std::collections::HashSet<String>,
    provider_health: std::collections::HashMap<String, domain::ConnectivityCheckResult>,
}

struct FormDraft {
    app: AppKind,
    editing_id: Option<String>,
    is_official: bool,
    name: Entity<InputState>,
    api_key: Entity<InputState>,
    base_url: Entity<InputState>,
    model: Entity<InputState>,
    zcode_modality_text: bool,
    zcode_modality_image: bool,
    workbuddy_supports_tool_call: bool,
    workbuddy_supports_images: bool,
    workbuddy_supports_reasoning: bool,
    workbuddy_reasoning_only: bool,
    workbuddy_can_disable_reasoning: bool,
    workbuddy_use_custom_protocol: bool,
    workbuddy_max_input_tokens: Entity<InputState>,
    workbuddy_max_output_tokens: Entity<InputState>,
    workbuddy_reasoning_effort: String,
    workbuddy_reasoning_effort_select:
        Option<Entity<SelectState<Vec<WorkBuddyReasoningEffortItem>>>>,
    workbuddy_supported_effort_low: bool,
    workbuddy_supported_effort_medium: bool,
    workbuddy_supported_effort_high: bool,
    workbuddy_supported_effort_xhigh: bool,
    workbuddy_supported_effort_max: bool,
    preset_select: Entity<SelectState<Vec<PresetSelectItem>>>,
    catalog_rows: Vec<CatalogRowDraft>,
    fetched_models: Vec<String>,
    has_fetched_models: bool,
    default_model_select: Option<Entity<SelectState<Vec<ModelSelectItem>>>>,
    is_fetching_models: bool,
    is_testing_connectivity: bool,
    connectivity_result: Option<domain::ConnectivityCheckResult>,
    _preset_sub: Option<Subscription>,
    _default_model_sub: Option<Subscription>,
    _workbuddy_reasoning_effort_sub: Option<Subscription>,
}

impl RouterApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let workspace = Workspace::open_default().expect("open workspace");
        let settings = workspace.settings().unwrap_or_default();
        rust_i18n::set_locale(settings.language.locale_str());
        crate::theme::apply_theme(settings.theme, Some(window), cx);

        let search_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t!("provider.search_placeholder").to_string())
        });

        let settings_search_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t!("settings.search_placeholder").to_string())
        });

        let window_items = vec![
            UsageWindowSelectItem {
                choice: UsageWindowChoice::Today,
                label: t!("usage.today").to_string(),
            },
            UsageWindowSelectItem {
                choice: UsageWindowChoice::Yesterday,
                label: t!("usage.yesterday").to_string(),
            },
            UsageWindowSelectItem {
                choice: UsageWindowChoice::Days7,
                label: t!("usage.days_7").to_string(),
            },
            UsageWindowSelectItem {
                choice: UsageWindowChoice::Days30,
                label: t!("usage.days_30").to_string(),
            },
            UsageWindowSelectItem {
                choice: UsageWindowChoice::Month,
                label: t!("usage.this_month").to_string(),
            },
        ];
        let usage_window_select = cx.new(|cx| {
            SelectState::new(
                window_items,
                Some(gpui_component::IndexPath::default().row(0)),
                window,
                cx,
            )
        });

        let usage_window_sub = cx.subscribe(
            &usage_window_select,
            |this: &mut RouterApp,
             _emitter: Entity<SelectState<Vec<UsageWindowSelectItem>>>,
             event: &SelectEvent<Vec<UsageWindowSelectItem>>,
             cx: &mut Context<Self>| {
                if let SelectEvent::Confirm(Some(choice)) = event {
                    this.usage_window = *choice;
                    this.refresh_dashboard_data(cx);
                }
            },
        );

        let refresh_items = vec![
            UsageRefreshSelectItem {
                interval: UsageRefreshInterval::Off,
                label: t!("usage.refresh_off").to_string(),
            },
            UsageRefreshSelectItem {
                interval: UsageRefreshInterval::Sec10,
                label: t!("usage.refresh_10s").to_string(),
            },
            UsageRefreshSelectItem {
                interval: UsageRefreshInterval::Sec30,
                label: t!("usage.refresh_30s").to_string(),
            },
            UsageRefreshSelectItem {
                interval: UsageRefreshInterval::Sec60,
                label: t!("usage.refresh_60s").to_string(),
            },
        ];
        let usage_refresh_select = cx.new(|cx| {
            SelectState::new(
                refresh_items,
                Some(gpui_component::IndexPath::default().row(2)),
                window,
                cx,
            )
        });

        let usage_refresh_sub = cx.subscribe(
            &usage_refresh_select,
            |this: &mut RouterApp,
             _emitter: Entity<SelectState<Vec<UsageRefreshSelectItem>>>,
             event: &SelectEvent<Vec<UsageRefreshSelectItem>>,
             cx: &mut Context<Self>| {
                if let SelectEvent::Confirm(Some(interval)) = event {
                    this.usage_refresh_interval = *interval;
                    cx.notify();
                }
            },
        );

        let log_config = settings.log_config;
        let log_level_select = cx.new(|cx| {
            SelectState::new(
                log_level_items(),
                Some(gpui_component::IndexPath::default().row(log_level_to_row(log_config.level))),
                window,
                cx,
            )
        });

        let log_level_sub = cx.subscribe(
            &log_level_select,
            |this: &mut RouterApp,
             _emitter: Entity<SelectState<Vec<LogLevelSelectItem>>>,
             event: &SelectEvent<Vec<LogLevelSelectItem>>,
             cx: &mut Context<Self>| {
                if let SelectEvent::Confirm(Some(level)) = event {
                    this.set_log_level(*level, cx);
                }
            },
        );

        let log_retention_select = cx.new(|cx| {
            SelectState::new(
                log_retention_items(),
                Some(
                    gpui_component::IndexPath::default()
                        .row(log_retention_to_row(log_config.retention_days)),
                ),
                window,
                cx,
            )
        });

        let log_retention_sub = cx.subscribe(
            &log_retention_select,
            |this: &mut RouterApp,
             _emitter: Entity<SelectState<Vec<LogRetentionSelectItem>>>,
             event: &SelectEvent<Vec<LogRetentionSelectItem>>,
             cx: &mut Context<Self>| {
                if let SelectEvent::Confirm(Some(days)) = event {
                    this.set_log_retention(*days, cx);
                }
            },
        );

        let supported = [
            "codex",
            "claude",
            "claude-desktop",
            "grok",
            "opencode",
            "pi",
            "cursor",
            "zcode",
            "workbuddy",
            "amp",
            "deepseek",
            "gemini",
            "fx",
            "hermes",
            "kimi",
            "ohmypi",
            "openclaw",
            "zai",
        ];
        let mut main_apps: Vec<String> = settings
            .main_apps
            .into_iter()
            .filter(|a| supported.contains(&a.as_str()))
            .collect();
        if main_apps.is_empty() {
            main_apps = vec![
                "codex".into(),
                "claude".into(),
                "grok".into(),
                "zcode".into(),
                "workbuddy".into(),
            ];
        }

        let mut app = Self {
            workspace,
            providers: Vec::new(),
            route: Route::Dashboard,
            previous_route: Route::Dashboard,
            sidebar_open: true,
            theme: settings.theme,
            language: settings.language,
            main_apps,
            launch_on_startup: settings.launch_on_startup,
            minimize_to_tray: settings.minimize_to_tray,
            settings_tab: SettingsTab::General,
            dashboard_app_filter: None,
            dashboard_data: None,
            is_loading_dashboard: false,
            usage_breakdown_tab: UsageBreakdownTab::Model,
            usage_window: UsageWindowChoice::Today,
            usage_metric: UsageMetric::Cost,
            usage_refresh_interval: UsageRefreshInterval::Sec30,
            usage_window_select,
            usage_refresh_select,
            _usage_window_sub: Some(usage_window_sub),
            _usage_refresh_sub: Some(usage_refresh_sub),
            log_config,
            log_level_select,
            log_retention_select,
            _log_level_sub: Some(log_level_sub),
            _log_retention_sub: Some(log_retention_sub),
            search_input,
            settings_search_input,
            last_error: None,
            form: None,
            logs: vec!["应用已启动并加载工作区".into()],
            env_tools: Vec::new(),
            is_inspecting_env: false,
            auto_check_update: settings.auto_check_update,
            skipped_update_version: settings.skipped_update_version,
            is_checking_update: false,
            testing_provider_ids: std::collections::HashSet::new(),
            provider_health: std::collections::HashMap::new(),
        };
        app.reload();
        app.refresh_dashboard_data(cx);

        if app.auto_check_update {
            let view_update = cx.entity().downgrade();
            window
                .spawn(cx, move |cx: &mut gpui::AsyncWindowContext| {
                    let mut cx = cx.clone();
                    async move {
                        cx.background_executor()
                            .timer(std::time::Duration::from_secs(3))
                            .await;
                        let _ = cx.update(|window: &mut Window, cx: &mut App| {
                            let _ = view_update.update(cx, |this, cx| {
                                this.check_for_updates(false, window, cx);
                            });
                        });
                    }
                })
                .detach();
        }

        let view = cx.entity().downgrade();
        let view_refresh = cx.entity().downgrade();
        window
            .spawn(cx, move |cx: &mut gpui::AsyncWindowContext| {
                let mut cx = cx.clone();
                async move {
                    let mut tick_ms: u64 = 0;
                    loop {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(150))
                            .await;
                        tick_ms += 150;
                        let mut pending = Vec::new();
                        if let Ok(rx) = crate::get_deeplink_channel().1.lock() {
                            while let Ok(url) = rx.try_recv() {
                                pending.push(url);
                            }
                        }
                        if !pending.is_empty() {
                            let _ = cx.update(|window: &mut Window, cx: &mut App| {
                                let _ = view.update(cx, |this, cx| {
                                    for url in pending {
                                        this.handle_deeplink_url(&url, window, cx);
                                    }
                                });
                            });
                        }

                        // Auto-refresh timer for dashboard every whole second
                        if tick_ms % 1000 == 0 {
                            let sec = tick_ms / 1000;
                            let _ = cx.update(|_window: &mut Window, cx: &mut App| {
                                let _ = view_refresh.update(cx, |this, cx| {
                                    if this.should_auto_refresh_dashboard(sec) {
                                        this.refresh_dashboard_data(cx);
                                    }
                                });
                            });
                        }
                    }
                }
            })
            .detach();

        app
    }

    pub fn refresh_dashboard_data(&mut self, cx: &mut Context<Self>) {
        let app_filter = self.dashboard_app_filter;
        let window_choice = self.usage_window;
        let metric = self.usage_metric;
        self.is_loading_dashboard = true;
        cx.notify();

        cx.spawn(
            move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
                let mut cx = cx.clone();
                async move {
                    let data = crate::usage_service::load_dashboard_usage(
                        app_filter,
                        window_choice,
                        metric,
                    )
                    .await;
                    let _ = this.update(
                        &mut cx,
                        |this: &mut RouterApp, cx: &mut Context<RouterApp>| {
                            this.dashboard_data = Some(data);
                            this.is_loading_dashboard = false;
                            cx.notify();
                        },
                    );
                }
            },
        )
        .detach();
    }

    pub fn should_auto_refresh_dashboard(&self, tick_seconds: u64) -> bool {
        if self.route != Route::Dashboard || self.is_loading_dashboard {
            return false;
        }
        match self.usage_refresh_interval {
            UsageRefreshInterval::Off => false,
            UsageRefreshInterval::Sec10 => tick_seconds % 10 == 0,
            UsageRefreshInterval::Sec30 => tick_seconds % 30 == 0,
            UsageRefreshInterval::Sec60 => tick_seconds % 60 == 0,
        }
    }

    pub fn handle_deeplink_url(&mut self, url: &str, window: &mut Window, cx: &mut Context<Self>) {
        let trimmed = url.trim();
        if trimmed.is_empty() {
            return;
        }

        match self.workspace.import_from_deeplink(trimmed) {
            Ok((provider, is_enabled)) => {
                self.reload();
                self.form = None;

                let target_route = match provider.app {
                    AppKind::Claude => Route::Claude,
                    AppKind::Codex => Route::Codex,
                    AppKind::Grok => Route::Grok,
                    AppKind::OpenCode => Route::OpenCode,
                    AppKind::Pi => Route::Pi,
                    AppKind::Cursor => Route::Cursor,
                    AppKind::ZCode => Route::ZCode,
                    AppKind::WorkBuddy => Route::WorkBuddy,
                };
                self.route = target_route;

                let app_name = provider.app.display_name();
                let msg = if is_enabled {
                    format!("成功导入并启用服务商「{}」({})", provider.name, app_name)
                } else {
                    format!("成功导入服务商「{}」({})", provider.name, app_name)
                };

                self.logs.push(format!("DeepLink: {}", msg));
                window.push_notification(Notification::success(msg), cx);
                cx.notify();
            }
            Err(err) => {
                let err_msg = format!("深链接导入失败: {err}");
                self.logs.push(format!("DeepLink Error: {}", err_msg));
                self.fail(err, window, cx);
            }
        }
    }

    fn reload(&mut self) {
        let mut all = Vec::new();
        for app in [
            AppKind::Codex,
            AppKind::Claude,
            AppKind::Grok,
            AppKind::OpenCode,
            AppKind::Pi,
            AppKind::Cursor,
            AppKind::ZCode,
            AppKind::WorkBuddy,
        ] {
            if let Ok(snapshot) = self.workspace.snapshot_for(app) {
                all.extend(snapshot.providers);
            }
        }
        self.providers = all;
        self.last_error = None;
    }

    fn current_id_for(&self, app: AppKind) -> Option<String> {
        self.workspace.snapshot_for(app).ok()?.current_id
    }

    fn providers_for(&self, app: AppKind) -> Vec<Provider> {
        self.providers
            .iter()
            .filter(|p| p.app == app)
            .cloned()
            .collect()
    }

    fn set_route(&mut self, route: Route, cx: &mut Context<Self>) {
        if self.route != route {
            self.previous_route = self.route;
            self.form = None;
        }
        self.route = route;
        cx.notify();
    }

    fn set_language(&mut self, language: AppLanguage, window: &mut Window, cx: &mut Context<Self>) {
        self.language = language;
        rust_i18n::set_locale(language.locale_str());
        if let Err(err) = self.workspace.set_language(language) {
            self.fail(err, window, cx);
            return;
        }

        let window_items = vec![
            UsageWindowSelectItem {
                choice: UsageWindowChoice::Today,
                label: t!("usage.today").to_string(),
            },
            UsageWindowSelectItem {
                choice: UsageWindowChoice::Yesterday,
                label: t!("usage.yesterday").to_string(),
            },
            UsageWindowSelectItem {
                choice: UsageWindowChoice::Days7,
                label: t!("usage.days_7").to_string(),
            },
            UsageWindowSelectItem {
                choice: UsageWindowChoice::Days30,
                label: t!("usage.days_30").to_string(),
            },
            UsageWindowSelectItem {
                choice: UsageWindowChoice::Month,
                label: t!("usage.this_month").to_string(),
            },
        ];
        self.usage_window_select.update(cx, |this, cx| {
            this.set_items(window_items, window, cx);
        });

        let refresh_items = vec![
            UsageRefreshSelectItem {
                interval: UsageRefreshInterval::Off,
                label: t!("usage.refresh_off").to_string(),
            },
            UsageRefreshSelectItem {
                interval: UsageRefreshInterval::Sec10,
                label: t!("usage.refresh_10s").to_string(),
            },
            UsageRefreshSelectItem {
                interval: UsageRefreshInterval::Sec30,
                label: t!("usage.refresh_30s").to_string(),
            },
            UsageRefreshSelectItem {
                interval: UsageRefreshInterval::Sec60,
                label: t!("usage.refresh_60s").to_string(),
            },
        ];
        self.usage_refresh_select.update(cx, |this, cx| {
            this.set_items(refresh_items, window, cx);
        });
        self.log_level_select.update(cx, |this, cx| {
            this.set_items(log_level_items(), window, cx);
        });
        self.log_retention_select.update(cx, |this, cx| {
            this.set_items(log_retention_items(), window, cx);
        });
        self.search_input.update(cx, |this, cx| {
            this.set_placeholder(t!("provider.search_placeholder").to_string(), window, cx);
        });
        self.settings_search_input.update(cx, |this, cx| {
            this.set_placeholder(t!("settings.search_placeholder").to_string(), window, cx);
        });

        let msg = match language {
            AppLanguage::ZhCn => "已切换界面语言为简体中文",
            AppLanguage::En => "Interface language set to English",
        };
        self.logs.push(msg.into());
        notify_success(msg, window, cx);
        cx.notify();
    }

    fn toggle_log_enabled(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut config = self.log_config.clone();
        config.enabled = !config.enabled;
        if let Err(err) = self.workspace.set_log_config(config.clone()) {
            self.fail(err, window, cx);
            return;
        }
        self.log_config = config;
        let msg = if self.log_config.enabled {
            "已启用应用诊断日志"
        } else {
            "已禁用应用诊断日志"
        };
        self.logs.push(msg.into());
        notify_info(msg, window, cx);
        cx.notify();
    }

    fn set_log_level(&mut self, level: store::LogLevel, cx: &mut Context<Self>) {
        if self.log_config.level == level {
            return;
        }
        let mut config = self.log_config.clone();
        config.level = level;
        if let Err(err) = self.workspace.set_log_config(config.clone()) {
            self.last_error = Some(err.to_string().into());
            cx.notify();
            return;
        }
        self.log_config = config;
        cx.notify();
    }

    fn set_log_retention(&mut self, days: u32, cx: &mut Context<Self>) {
        if self.log_config.retention_days == days {
            return;
        }
        let mut config = self.log_config.clone();
        config.retention_days = days;
        if let Err(err) = self.workspace.set_log_config(config.clone()) {
            self.last_error = Some(err.to_string().into());
            cx.notify();
            return;
        }
        self.log_config = config;
        cx.notify();
    }

    fn open_diagnostic_log_dir(&self, window: &mut Window, cx: &mut Context<Self>) {
        let log_dir = self.workspace.log_dir();
        let _ = std::fs::create_dir_all(&log_dir);
        if let Err(err) = session::reveal_path_in_explorer(&log_dir) {
            window.push_notification(
                Notification::error(format!(
                    "{}: {}",
                    t!("advanced.log_config.open_failed"),
                    err
                )),
                cx,
            );
        } else {
            window.push_notification(
                Notification::success(t!("advanced.log_config.opened").to_string()),
                cx,
            );
        }
    }

    fn clear_diagnostic_logs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.workspace.clear_logs() {
            Ok(_) => {
                window.push_notification(
                    Notification::success(t!("advanced.log_config.cleared").to_string()),
                    cx,
                );
            }
            Err(err) => {
                window.push_notification(
                    Notification::error(format!(
                        "{}: {}",
                        t!("advanced.log_config.clear_failed"),
                        err
                    )),
                    cx,
                );
            }
        }
        cx.notify();
    }

    fn toggle_main_app(&mut self, app_id: &str, _window: &mut Window, cx: &mut Context<Self>) {
        match self.workspace.toggle_main_app(app_id) {
            Ok(is_enabled) => {
                if is_enabled {
                    if !self.main_apps.iter().any(|a| a == app_id) {
                        self.main_apps.push(app_id.to_string());
                    }
                } else {
                    self.main_apps.retain(|a| a != app_id);
                }
                cx.notify();
            }
            Err(err) => self.fail(err, _window, cx),
        }
    }

    fn move_main_app(
        &mut self,
        source_id: &str,
        target_id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if source_id == target_id {
            return;
        }
        let Some(from_pos) = self.main_apps.iter().position(|id| id == source_id) else {
            return;
        };
        let Some(to_pos) = self.main_apps.iter().position(|id| id == target_id) else {
            return;
        };
        let item = self.main_apps.remove(from_pos);
        self.main_apps.insert(to_pos, item);
        if let Err(err) = self.workspace.reorder_main_apps(self.main_apps.clone()) {
            self.fail(err, window, cx);
            return;
        }
        cx.notify();
    }

    fn toggle_launch_on_startup(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let new_val = !self.launch_on_startup;
        self.launch_on_startup = new_val;
        if let Err(err) = self.workspace.set_launch_on_startup(new_val) {
            self.fail(err, window, cx);
            return;
        }
        let msg = if new_val {
            "已开启开机自启"
        } else {
            "已关闭开机自启"
        };
        self.logs.push(msg.into());
        notify_success(msg, window, cx);
        cx.notify();
    }

    fn toggle_minimize_to_tray(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let new_val = !self.minimize_to_tray;
        self.minimize_to_tray = new_val;
        if let Err(err) = self.workspace.set_minimize_to_tray(new_val) {
            self.fail(err, window, cx);
            return;
        }
        let msg = if new_val {
            "已开启关闭时最小化到托盘"
        } else {
            "已关闭最小化到托盘"
        };
        self.logs.push(msg.into());
        notify_success(msg, window, cx);
        cx.notify();
    }

    pub fn show_update_dialog(
        &self,
        release: AppRelease,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let current_version = env!("CARGO_PKG_VERSION").to_string();
        let auto_check = self.auto_check_update;
        let view_toggle = cx.entity().downgrade();
        let view_skip = cx.entity().downgrade();
        let view_install = cx.entity().downgrade();

        open_app_update_dialog(
            window,
            cx,
            release,
            current_version,
            auto_check,
            move |new_val, window, cx| {
                let _ = view_toggle.update(cx, |this, cx| {
                    this.toggle_auto_check_update(new_val, window, cx);
                });
            },
            move |version, window, cx| {
                let _ = view_skip.update(cx, |this, cx| {
                    this.skip_update_version(version, window, cx);
                });
            },
            move |url, _window, cx| {
                let _ = view_install.update(cx, |this, cx| {
                    this.logs.push(format!("已启动新版本下载: {url}"));
                    cx.notify();
                });
            },
        );
    }

    pub fn toggle_auto_check_update(
        &mut self,
        enabled: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.auto_check_update = enabled;
        if let Err(err) = self.workspace.set_auto_check_update(enabled) {
            self.fail(err, window, cx);
            return;
        }
        let msg = if enabled {
            "已开启自动检查更新"
        } else {
            "已关闭自动检查更新"
        };
        self.logs.push(msg.into());
        cx.notify();
    }

    pub fn skip_update_version(
        &mut self,
        version: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.skipped_update_version = Some(version.clone());
        if let Err(err) = self
            .workspace
            .set_skipped_update_version(Some(version.clone()))
        {
            self.fail(err, window, cx);
            return;
        }
        let msg = format!("已跳过版本 v{version} 的更新提示");
        self.logs.push(msg.clone());
        notify_success(&msg, window, cx);
        cx.notify();
    }

    pub fn check_for_updates(&mut self, manual: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_checking_update {
            return;
        }
        self.is_checking_update = true;
        cx.notify();

        if manual {
            window.push_notification(
                Notification::info(t!("update.checking_update").to_string()),
                cx,
            );
        }

        let curr_ver = env!("CARGO_PKG_VERSION").to_string();
        let skipped_ver = self.skipped_update_version.clone();
        let view = cx.entity().downgrade();

        window
            .spawn(cx, move |cx: &mut gpui::AsyncWindowContext| {
                let mut cx = cx.clone();
                async move {
                    let check_res = cx
                        .background_executor()
                        .spawn(async move { check_app_update("aohun/router-switch", &curr_ver) })
                        .await;

                    let _ = cx.update(|window: &mut Window, cx: &mut App| {
                        let _ = view.update(cx, |this, cx| {
                            this.is_checking_update = false;
                            match check_res {
                                Ok(Some(release)) => {
                                    // If skipped and not manual check, don't popup
                                    if !manual && skipped_ver.as_deref() == Some(&release.version) {
                                        cx.notify();
                                        return;
                                    }
                                    this.show_update_dialog(release, window, cx);
                                }
                                Ok(None) => {
                                    if manual {
                                        let msg = t!(
                                            "update.uptodate_tip",
                                            version = env!("CARGO_PKG_VERSION")
                                        )
                                        .to_string();
                                        window.push_notification(Notification::success(msg), cx);
                                    }
                                }
                                Err(err) => {
                                    if manual {
                                        let msg = t!("update.check_failed", error = err.as_str())
                                            .to_string();
                                        window.push_notification(Notification::warning(msg), cx);
                                    }
                                }
                            }
                            cx.notify();
                        });
                    });
                }
            })
            .detach();
    }

    pub fn preview_update_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let sample = sample_app_release(env!("CARGO_PKG_VERSION"));
        self.show_update_dialog(sample, window, cx);
    }

    fn refresh_env(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_inspecting_env {
            return;
        }
        self.is_inspecting_env = true;
        cx.notify();
        window.push_notification(Notification::info("正在检测本地环境并查询最新版本..."), cx);

        let view = cx.entity().downgrade();
        window
            .spawn(cx, move |cx: &mut gpui::AsyncWindowContext| {
                let mut cx = cx.clone();
                async move {
                    // 1. Fast local inspection in background thread (no UI freeze)
                    let local_tools = cx
                        .background_executor()
                        .spawn(async move { domain::inspect_all_tools(false) })
                        .await;
                    let _ = cx.update(|_window: &mut Window, cx: &mut App| {
                        let _ = view.update(cx, |this, cx| {
                            this.env_tools = local_tools;
                            cx.notify();
                        });
                    });

                    // 2. Full remote registry version fetch in background thread
                    let updated_tools = cx
                        .background_executor()
                        .spawn(async move { domain::inspect_all_tools(true) })
                        .await;
                    let _ = cx.update(|window: &mut Window, cx: &mut App| {
                        let _ = view.update(cx, |this, cx| {
                            this.env_tools = updated_tools;
                            this.is_inspecting_env = false;
                            window.push_notification(
                                Notification::success("本地 CLI 环境及最新版本检测完成"),
                                cx,
                            );
                            cx.notify();
                        });
                    });
                }
            })
            .detach();
    }

    fn diagnose_all_conflicts(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let conflict_count = self.env_tools.iter().filter(|t| t.has_conflicts).count();
        if conflict_count > 0 {
            let names = self
                .env_tools
                .iter()
                .filter(|t| t.has_conflicts)
                .map(|t| t.name.as_str())
                .collect::<Vec<_>>()
                .join("、");
            window.push_notification(
                Notification::warning(format!(
                    "诊断发现 {} 处工具存在多重安装（{}），命令行默认将采用标为「默认」的路径。",
                    conflict_count, names
                )),
                cx,
            );
        } else {
            window.push_notification(
                Notification::success("环境诊断完成：未检测到多重安装冲突"),
                cx,
            );
        }
    }

    fn run_tool_upgrade(&mut self, tool_id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let (cmd_desc, tool_name) = match tool_id {
            "claude" => ("npm i -g @anthropic-ai/claude-code@latest", "Claude Code"),
            "codex" => ("npm i -g @openai/codex@latest", "Codex"),
            "gemini" => ("npm i -g @google/gemini-cli@latest", "Gemini CLI"),
            "grok" => ("npm i -g @xai-official/grok@latest", "Grok Build"),
            "opencode" => ("npm i -g opencode-ai@latest", "OpenCode"),
            "pi" => ("npm i -g @earendil-works/pi-coding-agent@latest", "Pi"),
            "openclaw" => ("npm i -g openclaw@latest", "OpenClaw"),
            "hermes" => ("pip install -U hermes-agent", "Hermes"),
            _ => ("npm i -g latest", tool_id),
        };
        self.logs
            .push(format!("启动升级 {}: {}", tool_name, cmd_desc));
        window.push_notification(
            Notification::info(format!("已触发 {} 升级指令: {}", tool_name, cmd_desc)),
            cx,
        );
    }

    fn run_tool_install(&mut self, tool_id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let (cmd_desc, tool_name) = match tool_id {
            "claude" => ("npm i -g @anthropic-ai/claude-code", "Claude Code"),
            "codex" => ("npm i -g @openai/codex", "Codex"),
            "gemini" => ("npm i -g @google/gemini-cli", "Gemini CLI"),
            "grok" => ("npm i -g @xai-official/grok", "Grok Build"),
            "opencode" => ("npm i -g opencode-ai", "OpenCode"),
            "pi" => ("npm i -g @earendil-works/pi-coding-agent", "Pi"),
            "openclaw" => ("npm i -g openclaw", "OpenClaw"),
            "hermes" => ("pip install hermes-agent", "Hermes"),
            _ => ("npm i -g", tool_id),
        };
        self.logs
            .push(format!("触发安装 {}: {}", tool_name, cmd_desc));
        window.push_notification(
            Notification::info(format!("已提供 {} 安装指令: {}", tool_name, cmd_desc)),
            cx,
        );
    }

    fn enable(&mut self, provider_id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(provider) = self.providers.iter().find(|p| p.id == provider_id).cloned() else {
            return;
        };
        match self.workspace.enable(provider_id) {
            Ok(()) => {
                let provider_name = provider.name.clone();
                let is_official = provider.is_official();
                self.reload();
                self.logs.push(format!(
                    "已切换并启用 {} 服务商: {}",
                    provider.app.display_name(),
                    provider_name
                ));
                if is_official {
                    let hint = match provider.app {
                        AppKind::Codex => {
                            "已切到 Codex 官方。未登录时请在终端执行 codex login，然后重启 Codex。"
                        }
                        AppKind::Claude => {
                            "已切到 Claude Code 官方配置。可直接在终端使用官方 Claude Code 登录。"
                        }
                        AppKind::Grok => {
                            "已切到 Grok Build 官方配置。可直接使用官方 Grok CLI 认证。"
                        }
                        AppKind::OpenCode => {
                            "已切到 OpenCode 官方配置。可直接使用官方 OpenCode 认证。"
                        }
                        AppKind::Pi => "已切到 Pi 官方配置。可直接使用官方 Pi 认证。",
                        AppKind::Cursor => "已切到 Cursor 官方配置。",
                        AppKind::ZCode => "已切到 ZCode 官方配置。",
                        AppKind::WorkBuddy => "已切到 WorkBuddy 官方配置。",
                    };
                    notify_success(hint, window, cx);
                } else {
                    let hint = match provider.app {
                        AppKind::Codex => format!("已启用 {} 并写入 ~/.codex，请重启 Codex / 终端生效。", provider_name),
                        AppKind::Claude => format!("已启用 {} 并写入 ~/.claude/settings.json，请重启 Claude Code 生效。", provider_name),
                        AppKind::Grok => format!("已启用 {} 并写入 ~/.grok/config.toml，请重启 Grok Build 生效。", provider_name),
                        AppKind::OpenCode => format!("已启用 {} 并写入 ~/.config/opencode/opencode.json，请重启 OpenCode 生效。", provider_name),
                        AppKind::Pi => format!("已启用 {} 并写入 ~/.pi/agent/，请重启 Pi 生效。", provider_name),
                        AppKind::Cursor => format!("已启用 {} 并更新 Cursor 本地网关路由配置。", provider_name),
                        AppKind::ZCode => format!("已启用 {} 并写入 ~/.zcode/v2/config.json，请重启 ZCode 生效。", provider_name),
                        AppKind::WorkBuddy => format!("已启用 {} 并写入 ~/.workbuddy/models.json，请重启 WorkBuddy 生效。", provider_name),
                    };
                    notify_success(hint, window, cx);
                }
            }
            Err(error) => self.fail(error, window, cx),
        }
        cx.notify();
    }

    fn duplicate(&mut self, provider_id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let app = self
            .providers
            .iter()
            .find(|p| p.id == provider_id)
            .map(|p| p.app);
        match self.workspace.duplicate(provider_id) {
            Ok(_) => {
                self.reload();
                self.logs.push("复制了服务商配置".into());
                if app == Some(AppKind::ZCode) {
                    notify_success("已复制服务商配置并写入 ~/.zcode/v2/config.json", window, cx);
                } else if app == Some(AppKind::WorkBuddy) {
                    notify_success(
                        "已复制服务商配置并写入 ~/.workbuddy/models.json",
                        window,
                        cx,
                    );
                } else {
                    notify_success("已复制服务商配置", window, cx);
                }
            }
            Err(error) => self.fail(error, window, cx),
        }
        cx.notify();
    }

    fn confirm_delete(&mut self, provider_id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let id = provider_id.to_string();
        let view = cx.entity();
        let app = self.providers.iter().find(|p| p.id == id).map(|p| p.app);
        window.open_dialog(cx, move |dialog, _, _cx| {
            let target = id.clone();
            let view = view.clone();
            dialog
                .confirm()
                .title("确认删除服务商？")
                .child(if app == Some(AppKind::ZCode) {
                    "此操作将从 Router Switch 以及 ~/.zcode/v2/config.json 中移除该服务商。"
                } else if app == Some(AppKind::WorkBuddy) {
                    "此操作将从 Router Switch 以及 ~/.workbuddy/models.json 中移除该服务商。"
                } else {
                    "当前启用的服务商无法删除。此操作仅移除 Router Switch 中的记录。"
                })
                .on_ok(move |_, window, cx| {
                    let target = target.clone();
                    view.update(cx, |this, cx| {
                        match this.workspace.delete(&target) {
                            Ok(()) => {
                                this.reload();
                                this.logs.push(format!("删除了服务商: {}", target));
                                if app == Some(AppKind::ZCode) {
                                    notify_success(
                                        "服务商已成功删除并从 ZCode 配置文件中移除",
                                        window,
                                        cx,
                                    );
                                } else if app == Some(AppKind::WorkBuddy) {
                                    notify_success(
                                        "服务商已成功删除并从 WorkBuddy 配置文件中移除",
                                        window,
                                        cx,
                                    );
                                } else {
                                    notify_success("服务商已成功删除", window, cx);
                                }
                            }
                            Err(error) => this.fail(error, window, cx),
                        }
                        cx.notify();
                    });
                    true
                })
        });
    }

    fn test_provider_connectivity(
        &mut self,
        provider_id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.testing_provider_ids.contains(provider_id) {
            return;
        }

        let Some(provider) = self.providers.iter().find(|p| p.id == provider_id).cloned() else {
            return;
        };

        let target_id = provider_id.to_string();
        let target_name = provider.name.clone();
        self.testing_provider_ids.insert(target_id.clone());
        cx.notify();

        let view = cx.entity().downgrade();

        window
            .spawn(cx, move |cx: &mut gpui::AsyncWindowContext| {
                let mut cx = cx.clone();
                let provider = provider.clone();
                let target_id = target_id.clone();
                let target_name = target_name.clone();
                async move {
                    let result = cx
                        .background_executor()
                        .spawn(async move { domain::test_provider_connectivity(&provider) })
                        .await;

                    let _ = cx.update(|window: &mut Window, cx: &mut App| {
                        let _ = view.update(cx, |this, cx| {
                            this.testing_provider_ids.remove(&target_id);
                            this.provider_health
                                .insert(target_id.clone(), result.clone());

                            match result.status {
                                domain::HealthStatus::Operational => {
                                    let latency = result.latency_ms.unwrap_or(0);
                                    let msg = t!(
                                        "provider.reachable",
                                        name = target_name.as_str(),
                                        latency = latency
                                    )
                                    .to_string();
                                    this.logs.push(format!("[连通性测试] {}", msg));
                                    window.push_notification(Notification::success(msg), cx);
                                }
                                domain::HealthStatus::Degraded => {
                                    let latency = result.latency_ms.unwrap_or(0);
                                    let msg = t!(
                                        "provider.reachable_slow",
                                        name = target_name.as_str(),
                                        latency = latency
                                    )
                                    .to_string();
                                    this.logs.push(format!("[连通性测试] {}", msg));
                                    window.push_notification(Notification::warning(msg), cx);
                                }
                                domain::HealthStatus::Failed => {
                                    let msg = t!(
                                        "provider.unreachable",
                                        name = target_name.as_str(),
                                        error = result.message.as_str()
                                    )
                                    .to_string();
                                    this.logs.push(format!(
                                        "[连通性测试] {} ({})",
                                        msg,
                                        t!("provider.unreachable_hint")
                                    ));
                                    window.push_notification(Notification::error(msg), cx);
                                }
                            }
                            cx.notify();
                        });
                    });
                }
            })
            .detach();
    }

    fn test_form_connectivity(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(form) = self.form.as_mut() else {
            return;
        };
        let base_url = form.base_url.read(cx).value().to_string();
        let api_key = form.api_key.read(cx).value().to_string();
        let provider_name = form.name.read(cx).value().to_string();
        let display_name = if provider_name.trim().is_empty() {
            "当前服务商".to_string()
        } else {
            provider_name.trim().to_string()
        };

        if base_url.trim().is_empty() {
            window.push_notification(Notification::warning("请先填写 API 端点 (Base URL)"), cx);
            return;
        }

        form.is_testing_connectivity = true;
        form.connectivity_result = None;
        cx.notify();

        let view = cx.entity().downgrade();
        let target_name = display_name;

        window
            .spawn(cx, move |cx: &mut gpui::AsyncWindowContext| {
                let mut cx = cx.clone();
                let base_url = base_url.clone();
                let api_key = if api_key.trim().is_empty() {
                    None
                } else {
                    Some(api_key.trim().to_string())
                };

                async move {
                    let result = cx
                        .background_executor()
                        .spawn(async move {
                            let config = domain::ConnectivityCheckConfig::default();
                            domain::check_reachability_with_retry(
                                &base_url,
                                api_key.as_deref(),
                                &config,
                            )
                        })
                        .await;

                    let _ = cx.update(|window: &mut Window, cx: &mut App| {
                        let _ = view.update(cx, |this, cx| {
                            if let Some(form) = this.form.as_mut() {
                                form.is_testing_connectivity = false;
                                form.connectivity_result = Some(result.clone());
                            }

                            match result.status {
                                domain::HealthStatus::Operational => {
                                    let latency = result.latency_ms.unwrap_or(0);
                                    let msg = t!(
                                        "provider.reachable",
                                        name = target_name.as_str(),
                                        latency = latency
                                    )
                                    .to_string();
                                    this.logs.push(format!("[连通性测试] {}", msg));
                                    window.push_notification(Notification::success(msg), cx);
                                }
                                domain::HealthStatus::Degraded => {
                                    let latency = result.latency_ms.unwrap_or(0);
                                    let msg = t!(
                                        "provider.reachable_slow",
                                        name = target_name.as_str(),
                                        latency = latency
                                    )
                                    .to_string();
                                    this.logs.push(format!("[连通性测试] {}", msg));
                                    window.push_notification(Notification::warning(msg), cx);
                                }
                                domain::HealthStatus::Failed => {
                                    let msg = t!(
                                        "provider.unreachable",
                                        name = target_name.as_str(),
                                        error = result.message.as_str()
                                    )
                                    .to_string();
                                    this.logs.push(format!(
                                        "[连通性测试] {} ({})",
                                        msg,
                                        t!("provider.unreachable_hint")
                                    ));
                                    window.push_notification(Notification::error(msg), cx);
                                }
                            }
                            cx.notify();
                        });
                    });
                }
            })
            .detach();
    }

    fn open_create_form(&mut self, app: AppKind, window: &mut Window, cx: &mut Context<Self>) {
        let mut form = FormDraft::create(app, window, cx);
        let mut auto_filled = false;
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            if let Some(info) = parse_clipboard_provider_info(&text) {
                form.apply_clipboard_info(info, window, cx);
                auto_filled = true;
            }
        }
        self.form = Some(form);
        if auto_filled {
            self.logs
                .push("新建服务商：已从剪贴板自动识别并填入配置".into());
            window.push_notification(
                Notification::info(t!("provider.clipboard_auto_detected").to_string()),
                cx,
            );
        }
        cx.notify();
    }

    fn import_from_clipboard(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let clipboard_text = cx.read_from_clipboard().and_then(|item| item.text());
        let Some(text) = clipboard_text else {
            window.push_notification(
                Notification::warning(t!("provider.clipboard_imported_not_found").to_string()),
                cx,
            );
            return;
        };

        if let Some(info) = parse_clipboard_provider_info(&text) {
            if let Some(form) = self.form.as_mut() {
                form.apply_clipboard_info(info, window, cx);
                self.logs
                    .push("已从剪贴板成功识别并导入 API 端点与 Key".into());
                window.push_notification(
                    Notification::success(t!("provider.clipboard_imported_success").to_string()),
                    cx,
                );
                cx.notify();
            }
        } else {
            window.push_notification(
                Notification::warning(t!("provider.clipboard_imported_not_found").to_string()),
                cx,
            );
        }
    }

    fn open_edit_form(&mut self, provider_id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(provider) = self.providers.iter().find(|p| p.id == provider_id).cloned() else {
            return;
        };
        match self.workspace.form_for(provider_id) {
            Ok(form) => {
                self.form = Some(FormDraft::from_provider_form(
                    provider.app,
                    Some(provider_id.to_string()),
                    form,
                    window,
                    cx,
                ));
                cx.notify();
            }
            Err(error) => self.fail(error, window, cx),
        }
    }

    fn submit_form(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(form) = self.form.as_ref() else {
            return true;
        };
        let app = form.app;
        let editing_id = form.editing_id.clone();
        let payload = form.to_provider_form(cx);
        match self
            .workspace
            .save_form(app, editing_id.as_deref(), payload)
        {
            Ok(_) => {
                self.form = None;
                self.reload();
                self.logs
                    .push(format!("保存了 {} 服务商配置", app.display_name()));
                if app == AppKind::ZCode {
                    notify_success("服务商配置已保存并写入 ~/.zcode/v2/config.json", window, cx);
                } else if app == AppKind::WorkBuddy {
                    notify_success(
                        "服务商配置已保存并写入 ~/.workbuddy/models.json",
                        window,
                        cx,
                    );
                } else {
                    notify_success("服务商配置已保存", window, cx);
                }
                cx.notify();
                true
            }
            Err(error) => {
                self.fail(error, window, cx);
                false
            }
        }
    }

    fn apply_preset(
        &mut self,
        preset: PresetSelectItem,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(form) = self.form.as_mut() else {
            return;
        };
        form.is_official = preset.is_official;
        form.name
            .update(cx, |input, cx| input.set_value(preset.name, window, cx));
        form.base_url
            .update(cx, |input, cx| input.set_value(preset.base_url, window, cx));
        form.model
            .update(cx, |input, cx| input.set_value(preset.model, window, cx));
        if form.app == AppKind::ZCode {
            form.zcode_modality_text = preset.modality_text;
            form.zcode_modality_image = preset.modality_image;
        }
        if form.app == AppKind::WorkBuddy {
            if let Some(p) = WORKBUDDY_PRESETS.iter().find(|p| p.id == preset.id) {
                form.workbuddy_supports_tool_call = p.supports_tool_call;
                form.workbuddy_supports_images = p.supports_images;
                form.workbuddy_supports_reasoning = p.supports_reasoning;
                form.workbuddy_reasoning_only = p.reasoning_only;
                form.workbuddy_can_disable_reasoning = p.can_disable_reasoning;
                form.workbuddy_use_custom_protocol = p.use_custom_protocol;
                form.workbuddy_max_input_tokens.update(cx, |input, cx| {
                    input.set_value(
                        p.max_input_tokens
                            .map(|v| v.to_string())
                            .unwrap_or_default(),
                        window,
                        cx,
                    )
                });
                form.workbuddy_max_output_tokens.update(cx, |input, cx| {
                    input.set_value(
                        p.max_output_tokens
                            .map(|v| v.to_string())
                            .unwrap_or_default(),
                        window,
                        cx,
                    )
                });
                form.workbuddy_reasoning_effort = p.reasoning_effort.to_string();
                if let Some(ref effort_select) = form.workbuddy_reasoning_effort_select {
                    let options = ["low", "medium", "high", "xhigh", "max"];
                    let idx = options
                        .iter()
                        .position(|o| *o == p.reasoning_effort)
                        .map(|i| gpui_component::IndexPath::default().row(i));
                    effort_select.update(cx, |select, cx| {
                        select.set_selected_index(idx, window, cx);
                    });
                }
                form.workbuddy_supported_effort_low = p.reasoning_effort == "low";
                form.workbuddy_supported_effort_medium = true;
                form.workbuddy_supported_effort_high = p.reasoning_effort == "high";
                form.workbuddy_supported_effort_xhigh = false;
                form.workbuddy_supported_effort_max = false;
            }
        }
        cx.notify();
    }

    fn toggle_workbuddy_supports_tool_call(&mut self, cx: &mut Context<Self>) {
        if let Some(form) = self.form.as_mut() {
            form.workbuddy_supports_tool_call = !form.workbuddy_supports_tool_call;
            cx.notify();
        }
    }

    fn toggle_workbuddy_supports_images(&mut self, cx: &mut Context<Self>) {
        if let Some(form) = self.form.as_mut() {
            form.workbuddy_supports_images = !form.workbuddy_supports_images;
            cx.notify();
        }
    }

    fn toggle_workbuddy_supports_reasoning(&mut self, cx: &mut Context<Self>) {
        if let Some(form) = self.form.as_mut() {
            form.workbuddy_supports_reasoning = !form.workbuddy_supports_reasoning;
            cx.notify();
        }
    }

    fn toggle_workbuddy_reasoning_only(&mut self, cx: &mut Context<Self>) {
        if let Some(form) = self.form.as_mut() {
            form.workbuddy_reasoning_only = !form.workbuddy_reasoning_only;
            cx.notify();
        }
    }

    fn toggle_workbuddy_can_disable_reasoning(&mut self, cx: &mut Context<Self>) {
        if let Some(form) = self.form.as_mut() {
            form.workbuddy_can_disable_reasoning = !form.workbuddy_can_disable_reasoning;
            cx.notify();
        }
    }

    fn toggle_workbuddy_use_custom_protocol(&mut self, cx: &mut Context<Self>) {
        if let Some(form) = self.form.as_mut() {
            form.workbuddy_use_custom_protocol = !form.workbuddy_use_custom_protocol;
            cx.notify();
        }
    }

    fn toggle_workbuddy_supported_effort(&mut self, effort: &str, cx: &mut Context<Self>) {
        if let Some(form) = self.form.as_mut() {
            match effort {
                "low" => form.workbuddy_supported_effort_low = !form.workbuddy_supported_effort_low,
                "medium" => {
                    form.workbuddy_supported_effort_medium = !form.workbuddy_supported_effort_medium
                }
                "high" => {
                    form.workbuddy_supported_effort_high = !form.workbuddy_supported_effort_high
                }
                "xhigh" => {
                    form.workbuddy_supported_effort_xhigh = !form.workbuddy_supported_effort_xhigh
                }
                "max" => form.workbuddy_supported_effort_max = !form.workbuddy_supported_effort_max,
                _ => {}
            }
            cx.notify();
        }
    }

    fn toggle_zcode_modality_text(&mut self, cx: &mut Context<Self>) {
        if let Some(form) = self.form.as_mut() {
            form.zcode_modality_text = !form.zcode_modality_text;
            cx.notify();
        }
    }

    fn toggle_zcode_modality_image(&mut self, cx: &mut Context<Self>) {
        if let Some(form) = self.form.as_mut() {
            form.zcode_modality_image = !form.zcode_modality_image;
            cx.notify();
        }
    }

    fn fetch_models_for_form(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(form) = self.form.as_mut() else {
            return;
        };
        let base_url = form.base_url.read(cx).value().to_string();
        let api_key = form.api_key.read(cx).value().to_string();

        if base_url.trim().is_empty() {
            window.push_notification(Notification::warning("请先填写 API 端点 (Base URL)"), cx);
            return;
        }

        form.is_fetching_models = true;
        cx.notify();

        window.push_notification(Notification::info("正在从服务商获取模型列表..."), cx);

        let view = cx.entity().downgrade();
        window
            .spawn(cx, move |cx: &mut gpui::AsyncWindowContext| {
                let mut cx = cx.clone();
                let base_url = base_url.clone();
                let api_key = api_key.clone();
                async move {
                    let result: Result<Vec<String>, String> = cx
                        .background_executor()
                        .spawn(async move { domain::fetch_models_from_api(&base_url, &api_key) })
                        .await;

                    let _ = cx.update(|window: &mut Window, cx: &mut App| {
                        let _ = view.update(cx, |this, cx| {
                            if let Some(form) = this.form.as_mut() {
                                form.is_fetching_models = false;
                            }
                            match result {
                                Ok(models) => {
                                    let count = models.len();
                                    this.logs
                                        .push(format!("获取模型列表成功，共 {} 个模型", count));
                                    window.push_notification(
                                        Notification::success(format!(
                                            "获取成功！找到 {} 个可用模型",
                                            count
                                        )),
                                        cx,
                                    );

                                    if let Some(form) = this.form.as_mut() {
                                        form.fetched_models = models.clone();
                                        form.has_fetched_models = true;

                                        // Auto-set default model if empty or generic default
                                        let current_model = form.model.read(cx).value().to_string();
                                        if let Some(first) = models.first() {
                                            if current_model.trim().is_empty()
                                                || current_model == DEFAULT_CODEX_MODEL
                                                || current_model == DEFAULT_CLAUDE_MODEL
                                                || current_model == DEFAULT_GROK_MODEL
                                                || current_model == DEFAULT_OPENCODE_MODEL
                                                || current_model == DEFAULT_PI_MODEL
                                                || current_model == DEFAULT_CURSOR_MODEL
                                                || current_model == DEFAULT_ZCODE_MODEL
                                            {
                                                let first = first.clone();
                                                form.model.update(
                                                    cx,
                                                    |input: &mut InputState, cx| {
                                                        input.set_value(first, window, cx);
                                                    },
                                                );
                                            }
                                        }

                                        // Setup default model dropdown selector
                                        let items: Vec<ModelSelectItem> = models
                                            .iter()
                                            .map(|m| ModelSelectItem { name: m.clone() })
                                            .collect();
                                        let updated_model_val =
                                            form.model.read(cx).value().to_string();
                                        let selected_idx = models
                                            .iter()
                                            .position(|m| m == &updated_model_val)
                                            .map(|i| gpui_component::IndexPath::default().row(i));
                                        let default_select = cx.new(|cx| {
                                            SelectState::new(items, selected_idx, window, cx)
                                                .searchable(true)
                                        });
                                        let form_model_state = form.model.clone();
                                        let default_sub = window.subscribe(
                                            &default_select,
                                            cx,
                                            move |_,
                                                  event: &SelectEvent<Vec<ModelSelectItem>>,
                                                  window,
                                                  cx| {
                                                if let SelectEvent::Confirm(Some(m)) = event {
                                                    let val = m.clone();
                                                    form_model_state.update(cx, |input, cx| {
                                                        input.set_value(val, window, cx);
                                                    });
                                                }
                                            },
                                        );
                                        form.default_model_select = Some(default_select);
                                        form._default_model_sub = Some(default_sub);

                                        // Update existing catalog rows
                                        for row in &mut form.catalog_rows {
                                            row.set_fetched_models(&models, window, cx);
                                        }
                                    }
                                }
                                Err(err) => {
                                    this.logs.push(format!("获取模型列表失败: {}", err));
                                    window.push_notification(Notification::error(err), cx);
                                }
                            }
                            cx.notify();
                        });
                    });
                }
            })
            .detach();
    }

    fn add_catalog_row(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(form) = self.form.as_mut() {
            let fetched = form.fetched_models.clone();
            form.catalog_rows.push(CatalogRowDraft::new(
                "",
                "",
                Some(128_000),
                None,
                &fetched,
                window,
                cx,
            ));
            cx.notify();
        }
    }

    fn remove_catalog_row(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(form) = self.form.as_mut() {
            if index < form.catalog_rows.len() {
                form.catalog_rows.remove(index);
                cx.notify();
            }
        }
    }

    fn set_theme_preference(
        &mut self,
        theme: ThemePreference,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.theme = theme;
        if let Err(error) = self.workspace.set_theme(self.theme) {
            self.fail(error, window, cx);
            return;
        }
        crate::theme::apply_theme(self.theme, Some(window), cx);
        cx.notify();
    }

    fn toggle_cursor_gateway(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.workspace.is_cursor_gateway_running() {
            match self.workspace.stop_cursor_gateway() {
                Ok(()) => {
                    self.logs.push("Cursor 本地网关已停止".into());
                    notify_success("Cursor 本地网关已停止", window, cx);
                }
                Err(err) => self.fail(err, window, cx),
            }
        } else {
            match self.workspace.start_cursor_gateway(None, None) {
                Ok(_) => {
                    self.logs.push("Cursor 本地网关已启动".into());
                    notify_success("Cursor 本地网关已启动", window, cx);
                }
                Err(err) => self.fail(err, window, cx),
            }
        }
        cx.notify();
    }

    fn copy_cursor_ca_command(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let cmd = self.workspace.cursor_ca_install_command();
        cx.write_to_clipboard(gpui::ClipboardItem::new_string(cmd.clone()));
        self.logs.push(format!("已复制 CA 信任命令: {cmd}"));
        window.push_notification(Notification::info(t!("cursor.ca_copied").to_string()), cx);
        cx.notify();
    }

    fn fail(&mut self, error: impl ToString, window: &mut Window, cx: &mut Context<Self>) {
        let message = error.to_string();
        self.last_error = Some(message.clone().into());
        self.logs.push(format!("错误: {}", message));
        window.push_notification(Notification::error(message), cx);
        cx.notify();
    }

    fn filtered_providers(&self, app: AppKind, cx: &App) -> Vec<Provider> {
        let query = self.search_input.read(cx).value().to_string();
        let query = query.trim().to_lowercase();
        let app_providers = self.providers_for(app);
        if query.is_empty() {
            return app_providers;
        }

        app_providers
            .into_iter()
            .filter(|provider| {
                if provider.name.to_lowercase().contains(&query) {
                    return true;
                }
                match &provider.settings {
                    ProviderSettings::Codex(s) => {
                        extract_codex_model(&s.config_toml)
                            .is_some_and(|m| m.to_lowercase().contains(&query))
                            || extract_codex_base_url(&s.config_toml)
                                .is_some_and(|u| u.to_lowercase().contains(&query))
                    }
                    ProviderSettings::Claude(s) => {
                        extract_claude_model(&s.env)
                            .is_some_and(|m| m.to_lowercase().contains(&query))
                            || extract_claude_base_url(&s.env)
                                .is_some_and(|u| u.to_lowercase().contains(&query))
                    }
                    ProviderSettings::Grok(s) => {
                        extract_grok_model(&s.config_toml)
                            .is_some_and(|m| m.to_lowercase().contains(&query))
                            || extract_grok_base_url(&s.config_toml)
                                .is_some_and(|u| u.to_lowercase().contains(&query))
                    }
                    ProviderSettings::OpenCode(s) => {
                        extract_opencode_model(&s.models)
                            .is_some_and(|m| m.to_lowercase().contains(&query))
                            || extract_opencode_base_url(&s.options)
                                .is_some_and(|u| u.to_lowercase().contains(&query))
                    }
                    ProviderSettings::Pi(s) => {
                        s.model.to_lowercase().contains(&query)
                            || s.base_url.to_lowercase().contains(&query)
                    }
                    ProviderSettings::Cursor(s) => {
                        extract_cursor_model(s).is_some_and(|m| m.to_lowercase().contains(&query))
                            || extract_cursor_base_url(s)
                                .is_some_and(|u| u.to_lowercase().contains(&query))
                    }
                    ProviderSettings::ZCode(s) => {
                        extract_zcode_model(&s.models)
                            .is_some_and(|m| m.to_lowercase().contains(&query))
                            || extract_zcode_base_url(&s.options)
                                .is_some_and(|u| u.to_lowercase().contains(&query))
                    }
                    ProviderSettings::WorkBuddy(s) => {
                        s.model_id.to_lowercase().contains(&query)
                            || s.url.to_lowercase().contains(&query)
                            || s.vendor.to_lowercase().contains(&query)
                    }
                    ProviderSettings::Unsupported { .. } => false,
                }
            })
            .collect()
    }

    fn nav_item(
        &self,
        id: &'static str,
        icon: impl Into<Icon>,
        icon_color: Option<Hsla>,
        label: impl Into<SharedString>,
        route: Route,
        badge: Option<String>,
        disabled: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let active = self.route == route;
        let accent = cx.theme().sidebar_accent;
        let fg = cx.theme().sidebar_foreground;
        let label_str = label.into();

        div()
            .id(SharedString::from(id))
            .h(px(38.))
            .w_full()
            .px(px(10.))
            .rounded(px(8.))
            .flex()
            .items_center()
            .gap(px(10.))
            .text_size(px(14.))
            .text_color(if disabled { fg.opacity(0.4) } else { fg })
            .cursor_pointer()
            .when(active, |this| {
                this.bg(accent).font_weight(FontWeight::SEMIBOLD)
            })
            .when(!disabled, |this| this.hover(|this| this.bg(accent)))
            .child(
                Icon::new(icon)
                    .size(px(18.))
                    .flex_shrink_0()
                    .text_color(icon_color.unwrap_or(if active {
                        cx.theme().foreground
                    } else {
                        fg.opacity(0.85)
                    })),
            )
            .child(div().flex_1().truncate().child(label_str))
            .when_some(badge, |this, b| {
                this.child(
                    div()
                        .px(px(6.))
                        .py(px(1.))
                        .rounded(px(6.))
                        .bg(if active { cx.theme().primary } else { accent })
                        .text_size(px(11.))
                        .text_color(if active {
                            cx.theme().primary_foreground
                        } else {
                            fg.opacity(0.6)
                        })
                        .child(b),
                )
            })
            .when(!disabled, |this| {
                this.on_click(cx.listener(move |this, _, _, cx| this.set_route(route, cx)))
            })
    }

    fn draggable_nav_item(
        &self,
        app_id: &'static str,
        icon: impl Into<Icon>,
        icon_color: Option<Hsla>,
        label: &'static str,
        route: Route,
        badge: Option<String>,
        disabled: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let active = self.route == route;
        let accent = cx.theme().sidebar_accent;
        let fg = cx.theme().sidebar_foreground;
        let app_id_str = app_id.to_string();
        let target_app_id = app_id.to_string();

        div()
            .id(SharedString::from(format!("nav-app-{}", app_id)))
            .h(px(38.))
            .w_full()
            .px(px(10.))
            .rounded(px(8.))
            .flex()
            .items_center()
            .gap(px(10.))
            .text_size(px(14.))
            .text_color(if disabled { fg.opacity(0.4) } else { fg })
            .cursor_pointer()
            .when(active, |this| {
                this.bg(accent).font_weight(FontWeight::SEMIBOLD)
            })
            .when(!disabled, |this| this.hover(|this| this.bg(accent)))
            .child(
                Icon::new(icon)
                    .size(px(18.))
                    .flex_shrink_0()
                    .text_color(icon_color.unwrap_or(if active {
                        cx.theme().foreground
                    } else {
                        fg.opacity(0.85)
                    })),
            )
            .child(div().flex_1().truncate().child(label))
            .when_some(badge, |this, b| {
                this.child(
                    div()
                        .px(px(6.))
                        .py(px(1.))
                        .rounded(px(6.))
                        .bg(if active { cx.theme().primary } else { accent })
                        .text_size(px(11.))
                        .text_color(if active {
                            cx.theme().primary_foreground
                        } else {
                            fg.opacity(0.6)
                        })
                        .child(b),
                )
            })
            .when(!disabled, |this| {
                this.on_click(cx.listener(move |this, _, _, cx| this.set_route(route, cx)))
            })
            .on_drag(DragAppId(app_id_str), {
                let ghost_label = SharedString::from(label);
                move |_, _, _, cx| {
                    let label = ghost_label.clone();
                    cx.new(|_| DragGhostView { label })
                }
            })
            .on_drop(cx.listener(move |this, dragged: &DragAppId, window, cx| {
                this.move_main_app(&dragged.0, &target_app_id, window, cx);
            }))
    }

    fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let border = cx.theme().sidebar_border;
        let codex_count = self.providers_for(AppKind::Codex).len();
        let claude_count = self.providers_for(AppKind::Claude).len();
        let grok_count = self.providers_for(AppKind::Grok).len();
        let opencode_count = self.providers_for(AppKind::OpenCode).len();
        let pi_count = self.providers_for(AppKind::Pi).len();
        let cursor_count = self.providers_for(AppKind::Cursor).len();
        let zcode_count = self.providers_for(AppKind::ZCode).len();
        let workbuddy_count = self.providers_for(AppKind::WorkBuddy).len();

        let mut app_nav_items = Vec::new();
        for app_id in &self.main_apps {
            let item = match app_id.as_str() {
                "amp" => Some(self.draggable_nav_item(
                    "amp",
                    CustomIcon::Amp,
                    Some(rgb(0xEA580C).into()),
                    "Amp",
                    Route::Codex,
                    Some(t!("nav.soon").to_string()),
                    true,
                    cx,
                )),
                "claude" => Some(self.draggable_nav_item(
                    "claude",
                    CustomIcon::Claude,
                    Some(rgb(0xD97757).into()),
                    "Claude Code",
                    Route::Claude,
                    Some(format!("{claude_count}")),
                    false,
                    cx,
                )),
                "claude-desktop" => Some(self.draggable_nav_item(
                    "claude-desktop",
                    CustomIcon::Claude,
                    Some(rgb(0xD97757).into()),
                    "Claude Desktop",
                    Route::Claude,
                    Some(t!("nav.soon").to_string()),
                    true,
                    cx,
                )),
                "codex" => Some(self.draggable_nav_item(
                    "codex",
                    CustomIcon::OpenAI,
                    Some(rgb(0x10A37F).into()),
                    "Codex",
                    Route::Codex,
                    Some(format!("{codex_count}")),
                    false,
                    cx,
                )),
                "cursor" => Some(self.draggable_nav_item(
                    "cursor",
                    CustomIcon::Cursor,
                    Some(rgb(0x6366F1).into()),
                    "Cursor",
                    Route::Cursor,
                    Some(format!("{cursor_count}")),
                    false,
                    cx,
                )),
                "deepseek" | "gemini" => Some(self.draggable_nav_item(
                    "deepseek",
                    CustomIcon::DeepSeek,
                    Some(rgb(0x3B82F6).into()),
                    "DeepSeek Harness",
                    Route::Codex,
                    Some(t!("nav.soon").to_string()),
                    true,
                    cx,
                )),
                "fx" | "hermes" => Some(self.draggable_nav_item(
                    "fx",
                    CustomIcon::Fx,
                    Some(rgb(0x4B5563).into()),
                    "Fx",
                    Route::Codex,
                    Some(t!("nav.soon").to_string()),
                    true,
                    cx,
                )),
                "opencode" => Some(self.draggable_nav_item(
                    "opencode",
                    CustomIcon::OpenCode,
                    Some(rgb(0x0284C7).into()),
                    "OpenCode",
                    Route::OpenCode,
                    Some(format!("{opencode_count}")),
                    false,
                    cx,
                )),
                "grok" => Some(self.draggable_nav_item(
                    "grok",
                    CustomIcon::Grok,
                    Some(rgb(0x8B5CF6).into()),
                    "Grok Build",
                    Route::Grok,
                    Some(format!("{grok_count}")),
                    false,
                    cx,
                )),
                "kimi" => Some(self.draggable_nav_item(
                    "kimi",
                    CustomIcon::Kimi,
                    Some(rgb(0x2563EB).into()),
                    "Kimi Code",
                    Route::Codex,
                    Some(t!("nav.soon").to_string()),
                    true,
                    cx,
                )),
                "ohmypi" | "openclaw" => Some(self.draggable_nav_item(
                    "ohmypi",
                    CustomIcon::OhMyPi,
                    Some(rgb(0xEC4899).into()),
                    "Oh My Pi",
                    Route::Codex,
                    Some(t!("nav.soon").to_string()),
                    true,
                    cx,
                )),
                "pi" => Some(self.draggable_nav_item(
                    "pi",
                    CustomIcon::Pi,
                    Some(rgb(0x3B82F6).into()),
                    "Pi",
                    Route::Pi,
                    Some(format!("{pi_count}")),
                    false,
                    cx,
                )),
                "zcode" | "zai" => Some(self.draggable_nav_item(
                    "zcode",
                    CustomIcon::ZCode,
                    Some(rgb(0x10B981).into()),
                    "ZCode",
                    Route::ZCode,
                    Some(format!("{zcode_count}")),
                    false,
                    cx,
                )),
                "workbuddy" => Some(self.draggable_nav_item(
                    "workbuddy",
                    CustomIcon::WorkBuddy,
                    Some(rgb(0x06B6D4).into()),
                    "WorkBuddy",
                    Route::WorkBuddy,
                    Some(format!("{workbuddy_count}")),
                    false,
                    cx,
                )),
                _ => None,
            };
            if let Some(i) = item {
                app_nav_items.push(i);
            }
        }

        v_flex()
            .w(px(210.))
            .h_full()
            .flex_shrink_0()
            .p(px(8.))
            // Clear traffic lights and chrome bar
            .pt(px(48.))
            .gap(px(4.))
            .child(self.nav_item(
                "nav-dashboard",
                IconName::LayoutDashboard,
                Some(rgb(0x3B82F6).into()), // Blue
                t!("nav.dashboard").to_string(),
                Route::Dashboard,
                None,
                false,
                cx,
            ))
            .children(app_nav_items)
            .child(div().flex_1())
            .child(self.nav_item(
                "nav-notifications",
                IconName::Bell,
                Some(rgb(0xF59E0B).into()), // Amber
                t!("nav.notifications").to_string(),
                Route::Notifications,
                if self.logs.len() > 1 {
                    Some(format!("{}", self.logs.len()))
                } else {
                    None
                },
                false,
                cx,
            ))
            .child(div().h(px(1.)).mx(px(8.)).my(px(4.)).bg(border))
            .child(self.nav_item(
                "nav-settings",
                IconName::Settings,
                Some(rgb(0x64748B).into()), // Slate
                t!("nav.settings").to_string(),
                Route::Settings,
                None,
                false,
                cx,
            ))
    }

    fn chrome_button(
        &self,
        id: &'static str,
        icon: IconName,
        tooltip: &'static str,
        cx: &mut Context<Self>,
        on_click: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> impl IntoElement {
        Button::new(SharedString::new_static(id))
            .ghost()
            .xsmall()
            .icon(icon)
            .tooltip(tooltip)
            .on_click(cx.listener(move |this, _, window, cx| on_click(this, window, cx)))
    }

    fn render_chrome(&self, _window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let left_pad = if cfg!(target_os = "macos") { 96. } else { 16. };

        h_flex()
            .absolute()
            .top_0()
            .left_0()
            .right_0()
            .h(px(CHROME_HEIGHT))
            .items_center()
            .child(
                div()
                    .w(px(left_pad))
                    .h_full()
                    .window_control_area(WindowControlArea::Drag),
            )
            .child(h_flex().items_center().child(self.chrome_button(
                "chrome-sidebar",
                IconName::PanelLeft,
                "切换侧边栏 ⌘ B",
                cx,
                |this, _, cx| {
                    this.sidebar_open = !this.sidebar_open;
                    cx.notify();
                },
            )))
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .window_control_area(WindowControlArea::Drag),
            )
    }

    fn render_usage_line_chart(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let default_data = DashboardUsageData::default();
        let data = self.dashboard_data.as_ref().unwrap_or(&default_data);

        // Y-axis tick labels computed from actual data
        let (y_max_label, y_mid2_label, y_mid1_label, y_zero_label) =
            if self.usage_metric == UsageMetric::Cost {
                let max_c = (data.max_daily_cost * 1.15).max(1.0);
                (
                    format_currency(max_c),
                    format_currency(max_c * 0.66),
                    format_currency(max_c * 0.33),
                    "$0.00".to_string(),
                )
            } else {
                let max_t = ((data.max_daily_tokens as f64) * 1.15).max(1000.0) as i64;
                (
                    format_tokens(max_t),
                    format_tokens((max_t as f64 * 0.66) as i64),
                    format_tokens((max_t as f64 * 0.33) as i64),
                    "0".to_string(),
                )
            };

        // X-axis date labels matching the range
        let x_labels: Vec<String> = if data.daily_points.is_empty() {
            vec!["—".to_string()]
        } else if data.daily_points.len() == 1 {
            vec![data.daily_points[0].label.clone()]
        } else if data.daily_points.len() <= 3 {
            data.daily_points.iter().map(|p| p.label.clone()).collect()
        } else {
            let n = data.daily_points.len();
            vec![
                data.daily_points[0].label.clone(),
                data.daily_points[n / 2].label.clone(),
                data.daily_points[n - 1].label.clone(),
            ]
        };

        let daily_points = data.daily_points.clone();
        let metric = self.usage_metric;
        let max_val = if metric == UsageMetric::Cost {
            (data.max_daily_cost * 1.15).max(1.0)
        } else {
            ((data.max_daily_tokens as f64) * 1.15).max(1000.0)
        };

        let stroke_color = rgb(0x10A37F);

        theme::tile(cx).w_full().child(
            v_flex()
                .w_full()
                .gap(px(12.))
                // 1. Chart Top Bar (Title + [费用 | 令牌] Switcher + Legend)
                .child(
                    h_flex()
                        .w_full()
                        .justify_between()
                        .items_center()
                        .child(
                            div()
                                .text_size(px(14.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(theme.foreground)
                                .child(if self.usage_metric == UsageMetric::Cost {
                                    t!("usage.daily_cost").to_string()
                                } else {
                                    t!("usage.daily_tokens").to_string()
                                }),
                        )
                        .child(
                            h_flex()
                                .items_center()
                                .gap(px(16.))
                                // Segmented Switcher [ 费用 | 令牌 ]
                                .child(
                                    h_flex()
                                        .p(px(2.))
                                        .rounded(px(6.))
                                        .bg(theme.secondary.opacity(0.5))
                                        .border_1()
                                        .border_color(theme.border)
                                        .gap(px(2.))
                                        .child(
                                            Button::new("chart-metric-cost")
                                                .ghost()
                                                .xsmall()
                                                .selected(self.usage_metric == UsageMetric::Cost)
                                                .label(t!("usage.metric_cost_btn").to_string())
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.usage_metric = UsageMetric::Cost;
                                                    this.refresh_dashboard_data(cx);
                                                })),
                                        )
                                        .child(
                                            Button::new("chart-metric-tokens")
                                                .ghost()
                                                .xsmall()
                                                .selected(self.usage_metric == UsageMetric::Tokens)
                                                .label(t!("usage.metric_tokens_btn").to_string())
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.usage_metric = UsageMetric::Tokens;
                                                    this.refresh_dashboard_data(cx);
                                                })),
                                        ),
                                )
                                // Dynamic Tool Legends
                                .child(h_flex().items_center().gap(px(12.)).children(
                                    if data.active_clients.is_empty() {
                                        vec![
                                            h_flex()
                                                .items_center()
                                                .gap(px(5.))
                                                .child(
                                                    Icon::new(CustomIcon::Claude)
                                                        .size(px(14.))
                                                        .text_color(rgb(0xD97757)),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(12.))
                                                        .text_color(theme.muted_foreground)
                                                        .child("Claude Code"),
                                                )
                                                .into_any_element(),
                                            h_flex()
                                                .items_center()
                                                .gap(px(5.))
                                                .child(
                                                    Icon::new(CustomIcon::OpenAI)
                                                        .size(px(14.))
                                                        .text_color(rgb(0x10A37F)),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(12.))
                                                        .text_color(theme.muted_foreground)
                                                        .child("Codex"),
                                                )
                                                .into_any_element(),
                                        ]
                                    } else {
                                        data.active_clients
                                            .iter()
                                            .take(4)
                                            .map(|client| {
                                                let (icon, color, name): (
                                                    CustomIcon,
                                                    Hsla,
                                                    String,
                                                ) = match client.as_str() {
                                                    "claude" => (
                                                        CustomIcon::Claude,
                                                        rgb(0xD97757).into(),
                                                        "Claude Code".to_string(),
                                                    ),
                                                    "codex" => (
                                                        CustomIcon::OpenAI,
                                                        rgb(0x10A37F).into(),
                                                        "Codex".to_string(),
                                                    ),
                                                    "grok" => (
                                                        CustomIcon::Grok,
                                                        rgb(0x8B5CF6).into(),
                                                        "Grok".to_string(),
                                                    ),
                                                    "opencode" => (
                                                        CustomIcon::OpenCode,
                                                        rgb(0x6366F1).into(),
                                                        "OpenCode".to_string(),
                                                    ),
                                                    "pi" => (
                                                        CustomIcon::Pi,
                                                        rgb(0x10B981).into(),
                                                        "Pi".to_string(),
                                                    ),
                                                    "zcode" => (
                                                        CustomIcon::ZCode,
                                                        rgb(0x06B6D4).into(),
                                                        "ZCode".to_string(),
                                                    ),
                                                    "cursor" => (
                                                        CustomIcon::Cursor,
                                                        rgb(0x06B6D4).into(),
                                                        "Cursor".to_string(),
                                                    ),
                                                    "workbuddy" | "codebuddy" => (
                                                        CustomIcon::WorkBuddy,
                                                        rgb(0xF59E0B).into(),
                                                        "WorkBuddy".to_string(),
                                                    ),
                                                    "gemini" => (
                                                        CustomIcon::DeepSeek,
                                                        rgb(0x3B82F6).into(),
                                                        "Gemini".to_string(),
                                                    ),
                                                    other => (
                                                        CustomIcon::Activity,
                                                        rgb(0x64748B).into(),
                                                        other.to_string(),
                                                    ),
                                                };
                                                h_flex()
                                                    .items_center()
                                                    .gap(px(5.))
                                                    .child(
                                                        Icon::new(icon)
                                                            .size(px(14.))
                                                            .text_color(color),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_size(px(12.))
                                                            .text_color(theme.muted_foreground)
                                                            .child(name),
                                                    )
                                                    .into_any_element()
                                            })
                                            .collect()
                                    },
                                )),
                        ),
                )
                // 2. Plot Area: Y-Axis Ticks + Horizontal Grid Lines + Spline Area Canvas
                .child(
                    h_flex()
                        .w_full()
                        .gap(px(10.))
                        // Y-Axis Labels
                        .child(
                            v_flex()
                                .w(px(64.))
                                .h(px(170.))
                                .justify_between()
                                .items_end()
                                .text_size(px(11.5))
                                .text_color(theme.muted_foreground)
                                .child(div().child(y_max_label))
                                .child(div().child(y_mid2_label))
                                .child(div().child(y_mid1_label))
                                .child(div().child(y_zero_label)),
                        )
                        // Chart Canvas Area
                        .child(
                            v_flex()
                                .flex_1()
                                .h(px(170.))
                                .relative()
                                .overflow_hidden()
                                // Horizontal Grid Lines
                                .child(
                                    v_flex()
                                        .absolute()
                                        .top_0()
                                        .left_0()
                                        .right_0()
                                        .bottom_0()
                                        .justify_between()
                                        .child(
                                            div().w_full().h(px(1.)).bg(theme.border.opacity(0.45)),
                                        )
                                        .child(
                                            div().w_full().h(px(1.)).bg(theme.border.opacity(0.45)),
                                        )
                                        .child(
                                            div().w_full().h(px(1.)).bg(theme.border.opacity(0.45)),
                                        )
                                        .child(
                                            div()
                                                .w_full()
                                                .h(px(1.5))
                                                .bg(theme.foreground.opacity(0.85)),
                                        ),
                                )
                                // Dynamic Vector Spline Area Curve Canvas
                                .child(
                                    gpui::canvas(
                                        |_bounds, _window, _cx| (),
                                        move |bounds, _state, window, _cx| {
                                            if daily_points.is_empty() {
                                                return;
                                            }

                                            let n = daily_points.len();
                                            let top_pad = 10.0;
                                            let bot_pad = 10.0;
                                            let width_f32 = f32::from(bounds.size.width);
                                            let height_f32 = f32::from(bounds.size.height);
                                            let draw_h = (height_f32 - top_pad - bot_pad).max(10.0);

                                            let mut pts: Vec<gpui::Point<gpui::Pixels>> =
                                                Vec::with_capacity(n);
                                            for (i, pt) in daily_points.iter().enumerate() {
                                                let x_norm = if n > 1 {
                                                    i as f32 / (n - 1) as f32
                                                } else {
                                                    0.5
                                                };
                                                let x =
                                                    bounds.origin.x + gpui::px(x_norm * width_f32);
                                                let val = if metric == UsageMetric::Cost {
                                                    pt.cost
                                                } else {
                                                    pt.tokens as f64
                                                };
                                                let y_norm = (val / max_val).clamp(0.0, 1.0) as f32;
                                                let y = bounds.origin.y
                                                    + gpui::px(top_pad + draw_h * (1.0 - y_norm));
                                                pts.push(gpui::point(x, y));
                                            }

                                            if pts.len() == 1 {
                                                let p = pts[0];
                                                window.paint_quad(gpui::fill(
                                                    gpui::Bounds::new(
                                                        p - gpui::point(
                                                            gpui::px(4.0),
                                                            gpui::px(4.0),
                                                        ),
                                                        gpui::size(gpui::px(8.0), gpui::px(8.0)),
                                                    ),
                                                    stroke_color,
                                                ));
                                                return;
                                            }

                                            // Draw Area Fill under curve
                                            let mut fill_builder = gpui::PathBuilder::fill();
                                            let bottom_y = bounds.origin.y + bounds.size.height;
                                            fill_builder.move_to(gpui::point(pts[0].x, bottom_y));
                                            fill_builder.line_to(pts[0]);

                                            for i in 0..pts.len() - 1 {
                                                let p0 = pts[i];
                                                let p1 = pts[i + 1];
                                                let mid = gpui::point(
                                                    (p0.x + p1.x) / 2.0,
                                                    (p0.y + p1.y) / 2.0,
                                                );
                                                fill_builder.curve_to(mid, p0);
                                                fill_builder.curve_to(p1, mid);
                                            }

                                            fill_builder.line_to(gpui::point(
                                                pts[pts.len() - 1].x,
                                                bottom_y,
                                            ));
                                            fill_builder.line_to(gpui::point(pts[0].x, bottom_y));

                                            if let Ok(fill_path) = fill_builder.build() {
                                                window.paint_path(fill_path, rgba(0x10A37F25));
                                            }

                                            // Draw Stroke Curve
                                            let mut stroke_builder =
                                                gpui::PathBuilder::stroke(gpui::px(2.0));
                                            stroke_builder.move_to(pts[0]);
                                            for i in 0..pts.len() - 1 {
                                                let p0 = pts[i];
                                                let p1 = pts[i + 1];
                                                let mid = gpui::point(
                                                    (p0.x + p1.x) / 2.0,
                                                    (p0.y + p1.y) / 2.0,
                                                );
                                                stroke_builder.curve_to(mid, p0);
                                                stroke_builder.curve_to(p1, mid);
                                            }
                                            if let Ok(stroke_path) = stroke_builder.build() {
                                                window.paint_path(stroke_path, stroke_color);
                                            }

                                            // Draw Points
                                            for p in &pts {
                                                window.paint_quad(gpui::fill(
                                                    gpui::Bounds::new(
                                                        *p - gpui::point(
                                                            gpui::px(2.5),
                                                            gpui::px(2.5),
                                                        ),
                                                        gpui::size(gpui::px(5.0), gpui::px(5.0)),
                                                    ),
                                                    stroke_color,
                                                ));
                                            }
                                        },
                                    )
                                    .w_full()
                                    .h_full(),
                                ),
                        ),
                )
                // 3. X-Axis Labels below plot area
                .child(
                    h_flex()
                        .w_full()
                        .pl(px(74.))
                        .justify_between()
                        .text_size(px(11.5))
                        .text_color(theme.muted_foreground)
                        .children(x_labels.into_iter().map(|label| div().child(label))),
                ),
        )
    }

    fn render_dashboard_page(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let default_data = DashboardUsageData::default();
        let data = self.dashboard_data.as_ref().unwrap_or(&default_data);

        v_flex()
            .w_full()
            .gap(px(14.))
            // 1. CCSwitch Brand Filter Chips & Date / Refresh Selectors
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .p(px(8.))
                    .rounded(px(10.))
                    .bg(theme.secondary.opacity(0.35))
                    .border_1()
                    .border_color(theme.border)
                    .child(
                        // Left: Multi-App Brand Chips
                        h_flex()
                            .items_center()
                            .gap(px(6.))
                            // All
                            .child(
                                Button::new("filter-app-all")
                                    .ghost()
                                    .xsmall()
                                    .selected(self.dashboard_app_filter.is_none())
                                    .label(t!("usage.app_all").to_string())
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.dashboard_app_filter = None;
                                        this.refresh_dashboard_data(cx);
                                    })),
                            )
                            // Claude
                            .child(
                                Button::new("filter-app-claude")
                                    .ghost()
                                    .xsmall()
                                    .selected(self.dashboard_app_filter == Some(AppKind::Claude))
                                    .icon(CustomIcon::Claude)
                                    .label("Claude")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.dashboard_app_filter =
                                            if this.dashboard_app_filter == Some(AppKind::Claude) {
                                                None
                                            } else {
                                                Some(AppKind::Claude)
                                            };
                                        this.refresh_dashboard_data(cx);
                                    })),
                            )
                            // Codex
                            .child(
                                Button::new("filter-app-codex")
                                    .ghost()
                                    .xsmall()
                                    .selected(self.dashboard_app_filter == Some(AppKind::Codex))
                                    .icon(CustomIcon::OpenAI)
                                    .label("Codex")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.dashboard_app_filter =
                                            if this.dashboard_app_filter == Some(AppKind::Codex) {
                                                None
                                            } else {
                                                Some(AppKind::Codex)
                                            };
                                        this.refresh_dashboard_data(cx);
                                    })),
                            )
                            // Gemini
                            .child(
                                Button::new("filter-app-gemini")
                                    .ghost()
                                    .xsmall()
                                    .icon(CustomIcon::DeepSeek)
                                    .label("Gemini")
                                    .on_click(cx.listener(|_this, _, window, cx| {
                                        notify_info("Gemini 暂无近期用量记录", window, cx);
                                    })),
                            )
                            // Grok
                            .child(
                                Button::new("filter-app-grok")
                                    .ghost()
                                    .xsmall()
                                    .selected(self.dashboard_app_filter == Some(AppKind::Grok))
                                    .icon(CustomIcon::Grok)
                                    .label("Grok")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.dashboard_app_filter =
                                            if this.dashboard_app_filter == Some(AppKind::Grok) {
                                                None
                                            } else {
                                                Some(AppKind::Grok)
                                            };
                                        this.refresh_dashboard_data(cx);
                                    })),
                            )
                            // OpenCode
                            .child(
                                Button::new("filter-app-opencode")
                                    .ghost()
                                    .xsmall()
                                    .selected(self.dashboard_app_filter == Some(AppKind::OpenCode))
                                    .icon(CustomIcon::OpenCode)
                                    .label("OpenCode")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.dashboard_app_filter = if this.dashboard_app_filter
                                            == Some(AppKind::OpenCode)
                                        {
                                            None
                                        } else {
                                            Some(AppKind::OpenCode)
                                        };
                                        this.refresh_dashboard_data(cx);
                                    })),
                            )
                            // Pi
                            .child(
                                Button::new("filter-app-pi")
                                    .ghost()
                                    .xsmall()
                                    .selected(self.dashboard_app_filter == Some(AppKind::Pi))
                                    .icon(CustomIcon::Pi)
                                    .label("Pi")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.dashboard_app_filter =
                                            if this.dashboard_app_filter == Some(AppKind::Pi) {
                                                None
                                            } else {
                                                Some(AppKind::Pi)
                                            };
                                        this.refresh_dashboard_data(cx);
                                    })),
                            )
                            // ZCode
                            .child(
                                Button::new("filter-app-zcode")
                                    .ghost()
                                    .xsmall()
                                    .selected(self.dashboard_app_filter == Some(AppKind::ZCode))
                                    .icon(CustomIcon::ZCode)
                                    .label("ZCode")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.dashboard_app_filter =
                                            if this.dashboard_app_filter == Some(AppKind::ZCode) {
                                                None
                                            } else {
                                                Some(AppKind::ZCode)
                                            };
                                        this.refresh_dashboard_data(cx);
                                    })),
                            )
                            // WorkBuddy
                            .child(
                                Button::new("filter-app-workbuddy")
                                    .ghost()
                                    .xsmall()
                                    .selected(self.dashboard_app_filter == Some(AppKind::WorkBuddy))
                                    .icon(CustomIcon::WorkBuddy)
                                    .label("WorkBuddy")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.dashboard_app_filter = if this.dashboard_app_filter
                                            == Some(AppKind::WorkBuddy)
                                        {
                                            None
                                        } else {
                                            Some(AppKind::WorkBuddy)
                                        };
                                        this.refresh_dashboard_data(cx);
                                    })),
                            ),
                    )
                    .child(
                        // Right: Date Selector first, then Refresh Selector, and Refresh button
                        h_flex()
                            .items_center()
                            .gap(px(8.))
                            .child(
                                div()
                                    .w(px(115.))
                                    .child(Select::new(&self.usage_window_select).small()),
                            )
                            .child(
                                div()
                                    .w(px(80.))
                                    .child(Select::new(&self.usage_refresh_select).small()),
                            )
                            .child(
                                Button::new("dash-refresh-btn")
                                    .outline()
                                    .small()
                                    .icon(CustomIcon::RotateCw)
                                    .tooltip(t!("about.refresh").to_string())
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.refresh_dashboard_data(cx);
                                        let msg = if this.language == AppLanguage::En {
                                            "Dashboard stats refreshed"
                                        } else {
                                            "仪表盘数据已刷新"
                                        };
                                        notify_success(msg, window, cx);
                                    })),
                            ),
                    ),
            )
            // 2. 5-Tile Metric Strip (Processed, Cached Input, Uncached Input, Output, Cache Savings)
            .child(
                theme::tile(cx).p(px(0.)).overflow_hidden().child(
                    h_flex()
                        .w_full()
                        // Tile 1: Processed Tokens / Total Spend
                        .child(
                            v_flex()
                                .flex_1()
                                .p(px(12.))
                                .gap(px(2.))
                                .child(
                                    div()
                                        .text_size(px(11.5))
                                        .text_color(theme.muted_foreground)
                                        .child(if self.usage_metric == UsageMetric::Cost {
                                            t!("usage.total_cost").to_string()
                                        } else {
                                            t!("usage.processed_tokens").to_string()
                                        }),
                                )
                                .child(
                                    div()
                                        .text_size(px(17.))
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(theme.foreground)
                                        .child(if self.usage_metric == UsageMetric::Cost {
                                            data.total_cost_formatted.clone()
                                        } else {
                                            data.total_tokens_formatted.clone()
                                        }),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(theme.muted_foreground)
                                        .child(
                                            t!(
                                                "usage.per_active_day",
                                                count = data.per_active_day_formatted.as_str()
                                            )
                                            .to_string(),
                                        ),
                                ),
                        )
                        // Tile 2: Cached Input
                        .child(
                            v_flex()
                                .flex_1()
                                .p(px(12.))
                                .border_l_1()
                                .border_color(theme.border)
                                .gap(px(2.))
                                .child(
                                    div()
                                        .text_size(px(11.5))
                                        .text_color(theme.muted_foreground)
                                        .child(t!("usage.cached_input").to_string()),
                                )
                                .child(
                                    div()
                                        .text_size(px(17.))
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(theme.foreground)
                                        .child(data.cached_input_formatted.clone()),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(theme.muted_foreground)
                                        .child(
                                            t!(
                                                "usage.observed_input_share",
                                                share =
                                                    data.observed_input_share_formatted.as_str()
                                            )
                                            .to_string(),
                                        ),
                                ),
                        )
                        // Tile 3: Uncached Input
                        .child(
                            v_flex()
                                .flex_1()
                                .p(px(12.))
                                .border_l_1()
                                .border_color(theme.border)
                                .gap(px(2.))
                                .child(
                                    div()
                                        .text_size(px(11.5))
                                        .text_color(theme.muted_foreground)
                                        .child(t!("usage.uncached_input").to_string()),
                                )
                                .child(
                                    div()
                                        .text_size(px(17.))
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(theme.foreground)
                                        .child(data.uncached_input_formatted.clone()),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(theme.muted_foreground)
                                        .child(
                                            t!(
                                                "usage.cache_writes",
                                                count = data.cache_write_formatted.as_str()
                                            )
                                            .to_string(),
                                        ),
                                ),
                        )
                        // Tile 4: Output
                        .child(
                            v_flex()
                                .flex_1()
                                .p(px(12.))
                                .border_l_1()
                                .border_color(theme.border)
                                .gap(px(2.))
                                .child(
                                    div()
                                        .text_size(px(11.5))
                                        .text_color(theme.muted_foreground)
                                        .child(t!("usage.output_tokens").to_string()),
                                )
                                .child(
                                    div()
                                        .text_size(px(17.))
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(theme.foreground)
                                        .child(data.output_formatted.clone()),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(theme.muted_foreground)
                                        .child(
                                            t!(
                                                "usage.includes_reasoning",
                                                count = data.reasoning_formatted.as_str()
                                            )
                                            .to_string(),
                                        ),
                                ),
                        )
                        // Tile 5: Cache Savings
                        .child(
                            v_flex()
                                .flex_1()
                                .p(px(12.))
                                .border_l_1()
                                .border_color(theme.border)
                                .gap(px(2.))
                                .child(
                                    div()
                                        .text_size(px(11.5))
                                        .text_color(theme.muted_foreground)
                                        .child(t!("usage.cache_savings").to_string()),
                                )
                                .child(
                                    div()
                                        .text_size(px(17.))
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(rgb(0x10B981))
                                        .child(data.cache_savings_cost_formatted.clone()),
                                )
                                .child(
                                    div().text_size(px(11.)).text_color(rgb(0x10B981)).child(
                                        t!(
                                            "usage.raw_cost_multiple",
                                            multiple =
                                                data.cache_savings_multiple_formatted.as_str()
                                        )
                                        .to_string(),
                                    ),
                                ),
                        ),
                ),
            )
            // 3. Line Chart (折线图) directly below 5-Tile metric strip
            .child(self.render_usage_line_chart(cx))
            // 4. Detail Breakdown Table (Full-Width)
            .child(
                theme::tile(cx).w_full().child(
                    v_flex()
                        .w_full()
                        .gap(px(10.))
                        .child(
                            h_flex()
                                .w_full()
                                .justify_between()
                                .items_center()
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(theme.foreground)
                                        .child(t!("usage.breakdown_title").to_string()),
                                )
                                .child(
                                    h_flex()
                                        .p(px(2.))
                                        .rounded(px(7.))
                                        .bg(theme.secondary.opacity(0.5))
                                        .border_1()
                                        .border_color(theme.border)
                                        .gap(px(2.))
                                        .child(
                                            Button::new("tab-breakdown-model")
                                                .ghost()
                                                .xsmall()
                                                .selected(
                                                    self.usage_breakdown_tab
                                                        == UsageBreakdownTab::Model,
                                                )
                                                .label(t!("usage.tab_model"))
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.usage_breakdown_tab =
                                                        UsageBreakdownTab::Model;
                                                    cx.notify();
                                                })),
                                        )
                                        .child(
                                            Button::new("tab-breakdown-day")
                                                .ghost()
                                                .xsmall()
                                                .selected(
                                                    self.usage_breakdown_tab
                                                        == UsageBreakdownTab::Day,
                                                )
                                                .label(t!("usage.tab_day"))
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.usage_breakdown_tab =
                                                        UsageBreakdownTab::Day;
                                                    cx.notify();
                                                })),
                                        ),
                                ),
                        )
                        .child(match self.usage_breakdown_tab {
                            UsageBreakdownTab::Model => {
                                self.render_usage_daily_table(cx).into_any_element()
                            }
                            UsageBreakdownTab::Day => {
                                self.render_usage_day_table(cx).into_any_element()
                            }
                        }),
                ),
            )
    }

    fn render_cursor_gateway_banner(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let is_running = self.workspace.is_cursor_gateway_running();
        let proxy_port = self.workspace.cursor_proxy_port();
        let backend_port = self.workspace.cursor_backend_port();
        let ca_installed = self.workspace.cursor_ca_state();
        let theme = cx.theme();
        let dark = theme.is_dark();

        theme::tile(cx).child(
            v_flex()
                .w_full()
                .gap(px(12.))
                .child(
                    h_flex()
                        .w_full()
                        .items_center()
                        .justify_between()
                        .child(
                            h_flex()
                                .items_center()
                                .gap(px(10.))
                                .child(
                                    div()
                                        .size(px(34.))
                                        .rounded(px(8.))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .bg(if is_running {
                                            if dark {
                                                Hsla::from(rgba(0x10B98133))
                                            } else {
                                                Hsla::from(rgba(0x10B98126))
                                            }
                                        } else {
                                            theme.border.opacity(0.5)
                                        })
                                        .child(
                                            Icon::new(CustomIcon::Cursor).size(px(18.)).text_color(
                                                if is_running {
                                                    rgb(0x10B981).into()
                                                } else {
                                                    theme.muted_foreground
                                                },
                                            ),
                                        ),
                                )
                                .child(
                                    v_flex()
                                        .gap(px(2.))
                                        .child(
                                            h_flex()
                                                .items_center()
                                                .gap(px(8.))
                                                .child(
                                                    div()
                                                        .text_size(px(15.))
                                                        .font_weight(FontWeight::SEMIBOLD)
                                                        .text_color(theme.foreground)
                                                        .child(
                                                            t!("cursor.gateway_title").to_string(),
                                                        ),
                                                )
                                                .child(if is_running {
                                                    Tag::success().small().child(
                                                        t!("cursor.gateway_running").to_string(),
                                                    )
                                                } else {
                                                    Tag::secondary().small().child(
                                                        t!("cursor.gateway_stopped").to_string(),
                                                    )
                                                }),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .text_color(theme.muted_foreground)
                                                .child(t!("cursor.gateway_desc").to_string()),
                                        ),
                                ),
                        )
                        .child(
                            h_flex().items_center().gap(px(8.)).child(
                                Button::new("cursor-gateway-toggle-btn")
                                    .small()
                                    .when(is_running, |this| {
                                        this.outline().label(t!("cursor.stop_gateway").to_string())
                                    })
                                    .when(!is_running, |this| {
                                        this.primary().label(t!("cursor.start_gateway").to_string())
                                    })
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.toggle_cursor_gateway(window, cx);
                                    })),
                            ),
                        ),
                )
                .child(
                    h_flex()
                        .w_full()
                        .items_center()
                        .justify_between()
                        .p(px(10.))
                        .rounded(px(8.))
                        .bg(theme.secondary.opacity(0.35))
                        .border_1()
                        .border_color(theme.border)
                        .child(
                            h_flex()
                                .items_center()
                                .gap(px(16.))
                                .child(
                                    h_flex()
                                        .items_center()
                                        .gap(px(6.))
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .text_color(theme.muted_foreground)
                                                .child(format!("{}:", t!("cursor.proxy_port"))),
                                        )
                                        .child(
                                            div()
                                                .px(px(6.))
                                                .py(px(1.))
                                                .rounded(px(4.))
                                                .bg(theme.border)
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_size(px(11.))
                                                .text_color(theme.foreground)
                                                .child(format!("127.0.0.1:{proxy_port}")),
                                        ),
                                )
                                .child(
                                    h_flex()
                                        .items_center()
                                        .gap(px(6.))
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .text_color(theme.muted_foreground)
                                                .child(format!("{}:", t!("cursor.backend_port"))),
                                        )
                                        .child(
                                            div()
                                                .px(px(6.))
                                                .py(px(1.))
                                                .rounded(px(4.))
                                                .bg(theme.border)
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_size(px(11.))
                                                .text_color(theme.foreground)
                                                .child(format!("127.0.0.1:{backend_port}")),
                                        ),
                                )
                                .child(
                                    h_flex()
                                        .items_center()
                                        .gap(px(6.))
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .text_color(theme.muted_foreground)
                                                .child(format!("{}:", t!("cursor.ca_title"))),
                                        )
                                        .child(if ca_installed {
                                            Tag::success()
                                                .small()
                                                .child(t!("cursor.ca_installed").to_string())
                                        } else {
                                            Tag::warning()
                                                .small()
                                                .child(t!("cursor.ca_not_installed").to_string())
                                        }),
                                ),
                        )
                        .child(
                            Button::new("copy-ca-cmd-btn")
                                .ghost()
                                .small()
                                .icon(IconName::Copy)
                                .label(t!("cursor.ca_install_copy").to_string())
                                .tooltip(t!("cursor.ca_install_tip").to_string())
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.copy_cursor_ca_command(window, cx);
                                })),
                        ),
                ),
        )
    }

    fn render_app_providers_page(&self, app: AppKind, cx: &mut Context<Self>) -> impl IntoElement {
        let filtered = self.filtered_providers(app, cx);
        let app_name = app.display_name();

        v_flex()
            .w_full()
            .gap(px(14.))
            .when(app == AppKind::Cursor, |this| {
                this.child(self.render_cursor_gateway_banner(cx))
            })
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .gap(px(12.))
                    .child(
                        h_flex()
                            .flex_1()
                            .items_center()
                            .gap(px(8.))
                            .child(Input::new(&self.search_input).cleanable(true)),
                    )
                    .child(
                        Button::new(SharedString::from(format!("{}-add-top", app.as_str())))
                            .primary()
                            .icon(IconName::Plus)
                            .label(t!("provider.new").to_string())
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.open_create_form(app, window, cx);
                            })),
                    ),
            )
            .child(if filtered.is_empty() {
                let has_query = !self.search_input.read(cx).value().trim().is_empty();
                if has_query {
                    empty_search_state(cx).into_any_element()
                } else {
                    empty_state(app_name, cx).into_any_element()
                }
            } else {
                v_flex()
                    .w_full()
                    .gap(px(10.))
                    .children(
                        filtered
                            .into_iter()
                            .map(|provider| self.render_provider_card(&provider, cx)),
                    )
                    .into_any_element()
            })
    }

    fn render_provider_card(
        &self,
        provider: &Provider,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let is_direct_write_app =
            provider.app == AppKind::ZCode || provider.app == AppKind::WorkBuddy;
        let current_id = self.current_id_for(provider.app);
        let is_current = !is_direct_write_app && current_id.as_deref() == Some(&provider.id);
        let is_official = provider.is_official();
        let id = provider.id.clone();
        let website_url = provider.website_url.clone();

        let model = match &provider.settings {
            ProviderSettings::Codex(s) => {
                extract_codex_model(&s.config_toml).unwrap_or_else(|| "默认模型".into())
            }
            ProviderSettings::Claude(s) => {
                extract_claude_model(&s.env).unwrap_or_else(|| "默认模型".into())
            }
            ProviderSettings::Grok(s) => {
                extract_grok_model(&s.config_toml).unwrap_or_else(|| "默认模型".into())
            }
            ProviderSettings::OpenCode(s) => {
                extract_opencode_model(&s.models).unwrap_or_else(|| "默认模型".into())
            }
            ProviderSettings::Pi(s) => {
                if !s.model.is_empty() {
                    s.model.clone()
                } else {
                    "默认模型".into()
                }
            }
            ProviderSettings::Cursor(s) => {
                extract_cursor_model(s).unwrap_or_else(|| "默认模型".into())
            }
            ProviderSettings::ZCode(s) => {
                extract_zcode_model(&s.models).unwrap_or_else(|| "默认模型".into())
            }
            ProviderSettings::WorkBuddy(s) => {
                if !s.model_id.is_empty() {
                    s.model_id.clone()
                } else {
                    "默认模型".into()
                }
            }
            ProviderSettings::Unsupported { .. } => "-".into(),
        };

        let endpoint = match &provider.settings {
            ProviderSettings::Codex(s) => {
                extract_codex_base_url(&s.config_toml).unwrap_or_else(|| "官方端点 (OpenAI)".into())
            }
            ProviderSettings::Claude(s) => {
                extract_claude_base_url(&s.env).unwrap_or_else(|| "官方端点 (Anthropic)".into())
            }
            ProviderSettings::Grok(s) => {
                extract_grok_base_url(&s.config_toml).unwrap_or_else(|| "官方端点 (xAI)".into())
            }
            ProviderSettings::OpenCode(s) => extract_opencode_base_url(&s.options)
                .unwrap_or_else(|| "官方端点 (OpenCode)".into()),
            ProviderSettings::Pi(s) => {
                if !s.base_url.is_empty() {
                    s.base_url.clone()
                } else {
                    "官方端点 (Pi)".into()
                }
            }
            ProviderSettings::Cursor(s) => {
                extract_cursor_base_url(s).unwrap_or_else(|| "官方端点 (Cursor)".into())
            }
            ProviderSettings::ZCode(s) => {
                extract_zcode_base_url(&s.options).unwrap_or_else(|| "官方端点 (ZCode)".into())
            }
            ProviderSettings::WorkBuddy(s) => {
                if !s.url.is_empty() {
                    s.url.clone()
                } else {
                    "官方端点 (WorkBuddy)".into()
                }
            }
            ProviderSettings::Unsupported { .. } => "-".into(),
        };

        let login_type = if is_official {
            "官方认证 / OAuth"
        } else {
            "API Key"
        };

        let dark = cx.theme().is_dark();
        let card = theme::tile(cx);
        let card = if is_current {
            card.border_1()
                .border_color(if dark { rgb(0x3B82F6) } else { rgb(0x2563EB) })
                .bg(if dark {
                    rgba(0x1E3A8A26)
                } else {
                    rgba(0xEFF6FFFA)
                })
                .shadow_sm()
        } else {
            card
        };

        card.child(
            h_flex()
                .w_full()
                .items_center()
                .justify_between()
                .gap(px(12.))
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .gap(px(6.))
                        .child(
                            h_flex()
                                .items_center()
                                .gap(px(8.))
                                .child(
                                    div()
                                        .size(px(28.))
                                        .rounded(px(8.))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .bg(if is_current {
                                            if dark {
                                                rgb(0x3B82F6).into()
                                            } else {
                                                rgb(0x2563EB).into()
                                            }
                                        } else {
                                            cx.theme().border
                                        })
                                        .text_color(if is_current {
                                            rgb(0xFFFFFF).into()
                                        } else {
                                            cx.theme().foreground
                                        })
                                        .child(if is_official {
                                            IconName::Bot
                                        } else {
                                            IconName::SquareTerminal
                                        }),
                                )
                                .child(
                                    div()
                                        .text_size(px(15.))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(cx.theme().foreground)
                                        .child(provider.name.clone()),
                                )
                                .when(is_current, |this| {
                                    this.child(
                                        div()
                                            .px(px(6.))
                                            .py(px(1.))
                                            .rounded(px(6.))
                                            .bg(if dark { rgb(0x3B82F6) } else { rgb(0x2563EB) })
                                            .text_size(px(11.))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(rgb(0xFFFFFF))
                                            .child(t!("provider.in_use").to_string()),
                                    )
                                })
                                .child(if is_official {
                                    Tag::secondary()
                                        .small()
                                        .child(t!("provider.official_tag").to_string())
                                } else {
                                    Tag::info()
                                        .small()
                                        .child(t!("provider.third_party_tag").to_string())
                                })
                                .when_some(
                                    self.provider_health.get(&provider.id),
                                    |this, health| match health.status {
                                        domain::HealthStatus::Operational => {
                                            this.child(Tag::success().small().child(format!(
                                                "{}ms",
                                                health.latency_ms.unwrap_or(0)
                                            )))
                                        }
                                        domain::HealthStatus::Degraded => {
                                            this.child(Tag::warning().small().child(format!(
                                                "{}ms 较慢",
                                                health.latency_ms.unwrap_or(0)
                                            )))
                                        }
                                        domain::HealthStatus::Failed => {
                                            this.child(Tag::danger().small().child(
                                                t!("provider.connectivity_failed").to_string(),
                                            ))
                                        }
                                    },
                                ),
                        )
                        .child(
                            h_flex()
                                .items_center()
                                .gap(px(4.))
                                .child(
                                    Icon::new(IconName::Globe)
                                        .size(px(14.))
                                        .text_color(cx.theme().muted_foreground),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .text_color(cx.theme().foreground)
                                        .child(endpoint),
                                )
                                .when_some(
                                    website_url.filter(|s| !s.trim().is_empty()),
                                    |this, url| {
                                        let target = url.clone();
                                        this.child(
                                            Button::new(SharedString::from(format!(
                                                "web-{}",
                                                provider.id
                                            )))
                                            .ghost()
                                            .xsmall()
                                            .icon(IconName::ExternalLink)
                                            .tooltip(format!("官网：{}", target))
                                            .on_click(move |_, _, cx| {
                                                cx.open_url(&target);
                                            }),
                                        )
                                    },
                                ),
                        )
                        .child(
                            h_flex()
                                .items_center()
                                .gap(px(8.))
                                .child(
                                    h_flex()
                                        .items_center()
                                        .gap(px(2.))
                                        .child(
                                            div()
                                                .text_size(px(11.))
                                                .text_color(cx.theme().muted_foreground)
                                                .child("模型:"),
                                        )
                                        .child(Tag::secondary().small().child(model)),
                                )
                                .child(
                                    h_flex()
                                        .items_center()
                                        .gap(px(2.))
                                        .child(
                                            div()
                                                .text_size(px(11.))
                                                .text_color(cx.theme().muted_foreground)
                                                .child("凭证:"),
                                        )
                                        .child(
                                            Tag::secondary().small().outline().child(login_type),
                                        ),
                                ),
                        ),
                )
                .child(
                    h_flex()
                        .items_center()
                        .gap(px(6.))
                        .when(!is_direct_write_app, |this| {
                            this.child(
                                Button::new(SharedString::from(format!("enable-{}", provider.id)))
                                    .outline()
                                    .small()
                                    .icon(if is_current {
                                        IconName::Check
                                    } else {
                                        IconName::SquareTerminal
                                    })
                                    .label(if is_current {
                                        t!("provider.in_use").to_string()
                                    } else {
                                        t!("provider.enable").to_string()
                                    })
                                    .disabled(is_current)
                                    .on_click(cx.listener({
                                        let id = id.clone();
                                        move |this, _, window, cx| this.enable(&id, window, cx)
                                    })),
                            )
                        })
                        .child(
                            Button::new(SharedString::from(format!("edit-{}", provider.id)))
                                .outline()
                                .small()
                                .icon(IconName::Settings2)
                                .label(t!("provider.edit").to_string())
                                .on_click(cx.listener({
                                    let id = id.clone();
                                    move |this, _, window, cx| this.open_edit_form(&id, window, cx)
                                })),
                        )
                        .child(
                            Button::new(SharedString::from(format!("copy-{}", provider.id)))
                                .outline()
                                .small()
                                .icon(IconName::Copy)
                                .label(t!("provider.copy").to_string())
                                .on_click(cx.listener({
                                    let id = id.clone();
                                    move |this, _, window, cx| this.duplicate(&id, window, cx)
                                })),
                        )
                        .child({
                            let is_testing = self.testing_provider_ids.contains(&provider.id);
                            let id = id.clone();
                            Button::new(SharedString::from(format!("test-{}", provider.id)))
                                .outline()
                                .small()
                                .icon(CustomIcon::Activity)
                                .label(if is_testing {
                                    t!("provider.testing_connectivity").to_string()
                                } else {
                                    t!("provider.test_connectivity").to_string()
                                })
                                .tooltip("测试服务商 API 端点连通性")
                                .disabled(is_testing)
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.test_provider_connectivity(&id, window, cx);
                                }))
                        })
                        .child(
                            Button::new(SharedString::from(format!("delete-{}", provider.id)))
                                .outline()
                                .small()
                                .icon(IconName::Delete)
                                .label(t!("provider.delete").to_string())
                                .disabled(if is_direct_write_app {
                                    false
                                } else {
                                    is_current
                                })
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.confirm_delete(&id, window, cx)
                                })),
                        ),
                ),
        )
    }

    fn render_switch(&self, checked: bool, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let bg_color = if checked { theme.primary } else { theme.border };

        div()
            .w(px(38.))
            .h(px(22.))
            .rounded(px(11.))
            .bg(bg_color)
            .p(px(2.))
            .flex()
            .items_center()
            .child(
                div()
                    .size(px(18.))
                    .rounded(px(9.))
                    .bg(rgb(0xFFFFFF))
                    .shadow_xs()
                    .when(checked, |this| this.ml(px(16.)))
                    .when(!checked, |this| this.ml(px(0.))),
            )
    }

    fn render_settings_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let border = cx.theme().sidebar_border;
        let accent = cx.theme().sidebar_accent;
        let query = self.settings_search_input.read(cx).value().to_string();
        let query = query.trim().to_lowercase();

        let general_matches = query.is_empty()
            || "通用 general 界面 语言 简体中文 english language 外观 主题 浅色 深色 跟随系统 theme light dark system 主页面 显示 claude codex gemini grok opencode openclaw hermes pi amp cursor deepseek zcode fx kimi ohmypi 窗口行为 开机自启 startup 托盘 minimize tray"
                .contains(&query);

        let advanced_matches = query.is_empty()
            || "高级 advanced 诊断 日志 diagnostic log 级别 level 留存 retention 目录 清理 调试 debug trace info warn error"
                .contains(&query);

        let about_matches = query.is_empty()
            || "关于 about 版本 version aicwitch 环境 检查 诊断 升级 update cli github 官网 冲突"
                .contains(&query);

        v_flex()
            .w(px(210.))
            .h_full()
            .flex_shrink_0()
            .p(px(8.))
            .pt(px(48.))
            .gap(px(6.))
            .child(
                // Waku style Back button
                div()
                    .id("settings-back")
                    .h(px(34.))
                    .px(px(9.))
                    .rounded(px(8.))
                    .flex()
                    .items_center()
                    .gap(px(9.))
                    .cursor_pointer()
                    .text_size(px(13.))
                    .text_color(cx.theme().muted_foreground)
                    .hover(move |element| element.bg(accent))
                    .child(
                        Icon::new(CustomIcon::ArrowLeft)
                            .size(px(15.))
                            .text_color(cx.theme().muted_foreground),
                    )
                    .child(t!("settings.back").to_string())
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.route = this.previous_route;
                        cx.notify();
                    })),
            )
            .child(
                Input::new(&self.settings_search_input)
                    .small()
                    .cleanable(true),
            )
            .child(div().h(px(1.)).mx(px(4.)).my(px(2.)).bg(border))
            .child(
                v_flex()
                    .w_full()
                    .gap(px(3.))
                    .when(general_matches, |this| {
                        this.child(self.render_settings_sidebar_item(
                            SettingsTab::General,
                            IconName::Settings,
                            t!("settings.general").to_string(),
                            cx,
                        ))
                    })
                    .when(advanced_matches, |this| {
                        this.child(self.render_settings_sidebar_item(
                            SettingsTab::Advanced,
                            IconName::Settings2,
                            t!("settings.advanced").to_string(),
                            cx,
                        ))
                    })
                    .when(about_matches, |this| {
                        this.child(self.render_settings_sidebar_item(
                            SettingsTab::About,
                            IconName::Info,
                            t!("settings.about").to_string(),
                            cx,
                        ))
                    })
                    .when(
                        !general_matches && !advanced_matches && !about_matches,
                        |this| {
                            this.child(
                                div()
                                    .px(px(10.))
                                    .py(px(16.))
                                    .text_size(px(12.))
                                    .text_color(cx.theme().muted_foreground)
                                    .child(t!("settings.no_matching").to_string()),
                            )
                        },
                    ),
            )
    }

    fn render_settings_sidebar_item(
        &self,
        tab: SettingsTab,
        icon: impl Into<Icon>,
        label: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let active = self.settings_tab == tab;
        let accent = cx.theme().sidebar_accent;
        let fg = cx.theme().sidebar_foreground;
        let label_str = label.into();

        div()
            .id(SharedString::from(format!("settings-tab-{:?}", tab)))
            .h(px(36.))
            .w_full()
            .px(px(10.))
            .rounded(px(8.))
            .flex()
            .items_center()
            .gap(px(8.))
            .text_size(px(13.))
            .text_color(fg)
            .cursor_pointer()
            .when(active, |this| {
                this.bg(accent).font_weight(FontWeight::SEMIBOLD)
            })
            .when(!active, |this| {
                this.hover(|this| this.bg(accent.opacity(0.5)))
            })
            .child(
                Icon::new(icon)
                    .size(px(16.))
                    .flex_shrink_0()
                    .text_color(if active {
                        cx.theme().foreground
                    } else {
                        fg.opacity(0.7)
                    }),
            )
            .child(div().flex_1().truncate().child(label_str))
            .on_click(cx.listener(move |this, _, window, cx| {
                this.settings_tab = tab;
                if tab == SettingsTab::About && this.env_tools.is_empty() && !this.is_inspecting_env
                {
                    this.refresh_env(window, cx);
                }
                cx.notify();
            }))
    }

    fn render_app_toggle_chip(
        &self,
        id: &'static str,
        label: &'static str,
        icon: impl Into<Icon>,
        icon_color: Hsla,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let is_active = self.main_apps.iter().any(|a| a == id);
        let theme = cx.theme();

        div()
            .id(SharedString::from(format!("app-chip-{}", id)))
            .h(px(32.))
            .px(px(12.))
            .rounded(px(16.))
            .flex()
            .items_center()
            .gap(px(7.))
            .cursor_pointer()
            .text_size(px(12.))
            .font_weight(FontWeight::MEDIUM)
            .when(is_active, |this| {
                this.bg(rgb(0x2563EB)).text_color(rgb(0xFFFFFF)).shadow_xs()
            })
            .when(!is_active, |this| {
                this.bg(theme.secondary.opacity(0.5))
                    .text_color(theme.muted_foreground)
                    .border_1()
                    .border_color(theme.border)
                    .hover(|this| this.bg(theme.secondary))
            })
            .child(Icon::new(icon).size(px(14.)).text_color(if is_active {
                rgb(0xFFFFFF).into()
            } else {
                icon_color
            }))
            .child(label)
            .on_click(cx.listener(move |this, _, window, cx| {
                this.toggle_main_app(id, window, cx);
            }))
    }

    fn render_general_settings(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let query = self.settings_search_input.read(cx).value().to_string();
        let query = query.trim().to_lowercase();

        let lang_match = query.is_empty()
            || "界面语言 interface language 简体中文 英文 english 语言".contains(&query);
        let theme_match = query.is_empty()
            || "外观主题 appearance theme 浅色 深色 跟随系统 light dark system 主题"
                .contains(&query);
        let apps_match = query.is_empty()
            || "主页面显示 main page apps claude codex grok 侧边栏 导航".contains(&query);
        let window_match = query.is_empty()
            || "窗口行为 window behavior 开机自启 startup 关闭时最小化到托盘 minimize tray 托盘"
                .contains(&query);

        let none_match = !lang_match && !theme_match && !apps_match && !window_match;

        v_flex()
            .w_full()
            .gap(px(16.))
            .child(
                div()
                    .text_size(px(24.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme.foreground)
                    .child(t!("settings.general").to_string()),
            )
            .when(none_match, |this| {
                this.child(
                    theme::tile(cx).child(
                        v_flex()
                            .w_full()
                            .items_center()
                            .justify_center()
                            .py(px(32.))
                            .gap(px(8.))
                            .child(Icon::new(IconName::Search).size(px(20.)).text_color(theme.muted_foreground))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(theme.muted_foreground)
                                    .child(t!("general.no_matching_general").to_string()),
                            ),
                    ),
                )
            })
            // 1. 界面语言
            .when(lang_match, |this| {
                this.child(
                    theme::tile(cx).child(
                        v_flex()
                            .w_full()
                            .gap(px(10.))
                            .child(
                                v_flex()
                                    .gap(px(2.))
                                    .child(theme::tile_label(t!("general.interface_language").to_string(), cx))
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(theme.muted_foreground)
                                            .child(t!("general.interface_language_desc").to_string()),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .gap(px(8.))
                                    .child(
                                        Button::new("lang-zh")
                                            .outline()
                                            .small()
                                            .selected(self.language == AppLanguage::ZhCn)
                                            .label("简体中文")
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.set_language(AppLanguage::ZhCn, window, cx);
                                            })),
                                    )
                                    .child(
                                        Button::new("lang-en")
                                            .outline()
                                            .small()
                                            .selected(self.language == AppLanguage::En)
                                            .label("English")
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.set_language(AppLanguage::En, window, cx);
                                            })),
                                    ),
                            ),
                    ),
                )
            })
            // 2. 外观主题
            .when(theme_match, |this| {
                this.child(
                    theme::tile(cx).child(
                        v_flex()
                            .w_full()
                            .gap(px(10.))
                            .child(
                                v_flex()
                                    .gap(px(2.))
                                    .child(theme::tile_label(t!("general.appearance_theme").to_string(), cx))
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(theme.muted_foreground)
                                            .child(t!("general.appearance_theme_desc").to_string()),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .gap(px(8.))
                                    .child(
                                        Button::new("theme-lt")
                                            .outline()
                                            .small()
                                            .icon(IconName::Sun)
                                            .selected(self.theme == ThemePreference::Light)
                                            .label(t!("general.theme_light"))
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.set_theme_preference(ThemePreference::Light, window, cx);
                                            })),
                                    )
                                    .child(
                                        Button::new("theme-dk")
                                            .outline()
                                            .small()
                                            .icon(IconName::Moon)
                                            .selected(self.theme == ThemePreference::Dark)
                                            .label(t!("general.theme_dark"))
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.set_theme_preference(ThemePreference::Dark, window, cx);
                                            })),
                                    )
                                    .child(
                                        Button::new("theme-sys")
                                            .outline()
                                            .small()
                                            .icon(CustomIcon::Monitor)
                                            .selected(self.theme == ThemePreference::System)
                                            .label(t!("general.theme_system"))
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.set_theme_preference(ThemePreference::System, window, cx);
                                            })),
                                    ),
                            ),
                    ),
                )
            })
            // 3. 主页面显示
            .when(apps_match, |this| {
                this.child(
                    theme::tile(cx).child(
                        v_flex()
                            .w_full()
                            .gap(px(12.))
                            .child(
                                v_flex()
                                    .gap(px(2.))
                                    .child(theme::tile_label(t!("general.main_page_apps").to_string(), cx))
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(theme.muted_foreground)
                                            .child(t!("general.main_page_apps_desc").to_string()),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .w_full()
                                    .flex_wrap()
                                    .gap(px(8.))
                                    .child(self.render_app_toggle_chip("amp", "Amp", CustomIcon::Amp, rgb(0xEA580C).into(), cx))
                                    .child(self.render_app_toggle_chip("claude", "Claude Code", CustomIcon::Claude, rgb(0xD97757).into(), cx))
                                    .child(self.render_app_toggle_chip("claude-desktop", "Claude Desktop", CustomIcon::Claude, rgb(0xD97757).into(), cx))
                                    .child(self.render_app_toggle_chip("codex", "Codex", CustomIcon::OpenAI, rgb(0x10A37F).into(), cx))
                                    .child(self.render_app_toggle_chip("cursor", "Cursor CLI", CustomIcon::Cursor, rgb(0x6366F1).into(), cx))
                                    .child(self.render_app_toggle_chip("deepseek", "DeepSeek Harness", CustomIcon::DeepSeek, rgb(0x3B82F6).into(), cx))
                                    .child(self.render_app_toggle_chip("fx", "Fx", CustomIcon::Fx, rgb(0x4B5563).into(), cx))
                                    .child(self.render_app_toggle_chip("opencode", "OpenCode", CustomIcon::OpenCode, rgb(0x0284C7).into(), cx))
                                    .child(self.render_app_toggle_chip("grok", "Grok Build", CustomIcon::Grok, rgb(0x8B5CF6).into(), cx))
                                    .child(self.render_app_toggle_chip("kimi", "Kimi Code", CustomIcon::Kimi, rgb(0x2563EB).into(), cx))
                                    .child(self.render_app_toggle_chip("ohmypi", "Oh My Pi", CustomIcon::OhMyPi, rgb(0xEC4899).into(), cx))
                                    .child(self.render_app_toggle_chip("pi", "Pi", CustomIcon::Pi, rgb(0x3B82F6).into(), cx))
                                    .child(self.render_app_toggle_chip("zcode", "ZCode", CustomIcon::ZCode, rgb(0x10B981).into(), cx))
                                    .child(self.render_app_toggle_chip("workbuddy", "WorkBuddy", CustomIcon::WorkBuddy, rgb(0x06B6D4).into(), cx)),
                            ),
                    ),
                )
            })
            // 4. 窗口行为
            .when(window_match, |this| {
                this.child(
                    theme::tile(cx).child(
                        v_flex()
                            .w_full()
                            .gap(px(12.))
                            .child(
                                v_flex()
                                    .gap(px(2.))
                                    .child(theme::tile_label(t!("general.window_behavior").to_string(), cx))
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(theme.muted_foreground)
                                            .child(t!("general.launch_on_startup_desc").to_string()),
                                    ),
                            )
                            .child(
                                // 开机自启
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .p(px(8.))
                                    .rounded(px(8.))
                                    .bg(theme.secondary.opacity(0.4))
                                    .child(
                                        h_flex()
                                            .items_center()
                                            .gap(px(10.))
                                            .child(
                                                div()
                                                    .size(px(32.))
                                                    .rounded(px(8.))
                                                    .bg(theme.border)
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .child(Icon::new(IconName::Settings2).size(px(16.))),
                                            )
                                            .child(
                                                v_flex()
                                                    .gap(px(2.))
                                                    .child(
                                                        div()
                                                            .text_size(px(13.))
                                                            .font_weight(FontWeight::MEDIUM)
                                                            .text_color(theme.foreground)
                                                            .child(t!("general.launch_on_startup").to_string()),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_size(px(11.))
                                                            .text_color(theme.muted_foreground)
                                                            .child(t!("general.launch_on_startup_desc").to_string()),
                                                    ),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .id("switch-launch-on-startup")
                                            .cursor_pointer()
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.toggle_launch_on_startup(window, cx);
                                            }))
                                            .child(self.render_switch(self.launch_on_startup, cx)),
                                    ),
                            )
                            .child(
                                // 关闭时最小化到托盘
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .p(px(8.))
                                    .rounded(px(8.))
                                    .bg(theme.secondary.opacity(0.4))
                                    .child(
                                        h_flex()
                                            .items_center()
                                            .gap(px(10.))
                                            .child(
                                                div()
                                                    .size(px(32.))
                                                    .rounded(px(8.))
                                                    .bg(theme.border)
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .child(Icon::new(IconName::WindowMinimize).size(px(16.))),
                                            )
                                            .child(
                                                v_flex()
                                                    .gap(px(2.))
                                                    .child(
                                                        div()
                                                            .text_size(px(13.))
                                                            .font_weight(FontWeight::MEDIUM)
                                                            .text_color(theme.foreground)
                                                            .child(t!("general.minimize_to_tray").to_string()),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_size(px(11.))
                                                            .text_color(theme.muted_foreground)
                                                            .child(t!("general.minimize_to_tray_desc").to_string()),
                                                    ),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .id("switch-minimize-to-tray")
                                            .cursor_pointer()
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.toggle_minimize_to_tray(window, cx);
                                            }))
                                            .child(self.render_switch(self.minimize_to_tray, cx)),
                                    ),
                            )
                            .child(
                                // 自动检查应用更新
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .p(px(8.))
                                    .rounded(px(8.))
                                    .bg(theme.secondary.opacity(0.4))
                                    .child(
                                        h_flex()
                                            .items_center()
                                            .gap(px(10.))
                                            .child(
                                                div()
                                                    .size(px(32.))
                                                    .rounded(px(8.))
                                                    .bg(theme.border)
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .child(Icon::new(CustomIcon::RotateCw).size(px(16.))),
                                            )
                                            .child(
                                                v_flex()
                                                    .gap(px(2.))
                                                    .child(
                                                        div()
                                                            .text_size(px(13.))
                                                            .font_weight(FontWeight::MEDIUM)
                                                            .text_color(theme.foreground)
                                                            .child(t!("update.auto_check_settings").to_string()),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_size(px(11.))
                                                            .text_color(theme.muted_foreground)
                                                            .child(t!("update.auto_check_settings_desc").to_string()),
                                                    ),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .id("switch-auto-check-update")
                                            .cursor_pointer()
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                let new_val = !this.auto_check_update;
                                                this.toggle_auto_check_update(new_val, window, cx);
                                            }))
                                            .child(self.render_switch(self.auto_check_update, cx)),
                                    ),
                            ),
                    ),
                )
            })
    }

    fn render_usage_daily_table(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let default_data = DashboardUsageData::default();
        let data = self.dashboard_data.as_ref().unwrap_or(&default_data);

        v_flex()
            .w_full()
            .text_size(px(12.5))
            .child(
                h_flex()
                    .w_full()
                    .pb(px(8.))
                    .border_b_1()
                    .border_color(theme.border)
                    .text_color(theme.muted_foreground)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(t!("usage.col_model").to_string()),
                    )
                    .child(
                        div().w(px(84.)).child(
                            h_flex()
                                .justify_end()
                                .w_full()
                                .child(t!("usage.col_cost").to_string()),
                        ),
                    )
                    .child(
                        div().w(px(64.)).child(
                            h_flex()
                                .justify_end()
                                .w_full()
                                .child(t!("usage.col_share").to_string()),
                        ),
                    )
                    .child(
                        div().w(px(84.)).child(
                            h_flex()
                                .justify_end()
                                .w_full()
                                .child(t!("usage.col_tokens").to_string()),
                        ),
                    ),
            )
            .child(if data.model_rows.is_empty() {
                div()
                    .w_full()
                    .py(px(24.))
                    .flex()
                    .justify_center()
                    .items_center()
                    .text_color(theme.muted_foreground)
                    .child(if self.is_loading_dashboard {
                        t!("usage.loading").to_string()
                    } else {
                        t!("usage.no_records").to_string()
                    })
                    .into_any_element()
            } else {
                v_flex()
                    .w_full()
                    .children(data.model_rows.iter().map(|row| {
                        h_flex()
                            .w_full()
                            .py(px(7.))
                            .border_b_1()
                            .border_color(theme.border.opacity(0.4))
                            .items_center()
                            .child(
                                h_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .items_center()
                                    .gap(px(7.))
                                    .child(
                                        Icon::new(row.app_icon).size(px(14.)).text_color(row.color),
                                    )
                                    .child(
                                        div()
                                            .truncate()
                                            .text_color(theme.foreground)
                                            .child(row.display_name.clone()),
                                    ),
                            )
                            .child(
                                div().w(px(84.)).child(
                                    h_flex()
                                        .justify_end()
                                        .w_full()
                                        .text_color(theme.foreground)
                                        .child(row.cost_formatted.clone()),
                                ),
                            )
                            .child(
                                div().w(px(64.)).child(
                                    h_flex()
                                        .justify_end()
                                        .w_full()
                                        .text_color(theme.muted_foreground)
                                        .child(row.share_formatted.clone()),
                                ),
                            )
                            .child(
                                div().w(px(84.)).child(
                                    h_flex()
                                        .justify_end()
                                        .w_full()
                                        .text_color(theme.muted_foreground)
                                        .child(row.tokens_formatted.clone()),
                                ),
                            )
                    }))
                    .into_any_element()
            })
    }

    fn render_usage_day_table(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let default_data = DashboardUsageData::default();
        let data = self.dashboard_data.as_ref().unwrap_or(&default_data);

        v_flex()
            .w_full()
            .text_size(px(12.5))
            .child(
                h_flex()
                    .w_full()
                    .pb(px(8.))
                    .border_b_1()
                    .border_color(theme.border)
                    .text_color(theme.muted_foreground)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(t!("usage.col_date").to_string()),
                    )
                    .child(
                        div().w(px(84.)).child(
                            h_flex()
                                .justify_end()
                                .w_full()
                                .child(t!("usage.col_cost").to_string()),
                        ),
                    )
                    .child(
                        div().w(px(64.)).child(
                            h_flex()
                                .justify_end()
                                .w_full()
                                .child(t!("usage.col_share").to_string()),
                        ),
                    )
                    .child(
                        div().w(px(84.)).child(
                            h_flex()
                                .justify_end()
                                .w_full()
                                .child(t!("usage.col_tokens").to_string()),
                        ),
                    ),
            )
            .child(if data.day_rows.is_empty() {
                div()
                    .w_full()
                    .py(px(24.))
                    .flex()
                    .justify_center()
                    .items_center()
                    .text_color(theme.muted_foreground)
                    .child(if self.is_loading_dashboard {
                        t!("usage.loading").to_string()
                    } else {
                        t!("usage.no_records").to_string()
                    })
                    .into_any_element()
            } else {
                v_flex()
                    .w_full()
                    .children(data.day_rows.iter().map(|row| {
                        h_flex()
                            .w_full()
                            .py(px(7.))
                            .border_b_1()
                            .border_color(theme.border.opacity(0.4))
                            .items_center()
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_color(theme.foreground)
                                    .child(row.date_display.clone()),
                            )
                            .child(
                                div().w(px(84.)).child(
                                    h_flex()
                                        .justify_end()
                                        .w_full()
                                        .text_color(theme.foreground)
                                        .child(row.cost_formatted.clone()),
                                ),
                            )
                            .child(
                                div().w(px(64.)).child(
                                    h_flex()
                                        .justify_end()
                                        .w_full()
                                        .text_color(theme.muted_foreground)
                                        .child(row.share_formatted.clone()),
                                ),
                            )
                            .child(
                                div().w(px(84.)).child(
                                    h_flex()
                                        .justify_end()
                                        .w_full()
                                        .text_color(theme.muted_foreground)
                                        .child(row.tokens_formatted.clone()),
                                ),
                            )
                    }))
                    .into_any_element()
            })
    }

    fn render_advanced_settings(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let query = self.settings_search_input.read(cx).value().to_string();
        let query = query.trim().to_lowercase();

        let log_match = query.is_empty()
            || "高级 advanced 诊断 日志 diagnostic log 级别 level 留存 retention 目录 清理 调试 debug trace info warn error"
                .contains(&query);

        v_flex()
            .w_full()
            .gap(px(16.))
            .child(
                div()
                    .text_size(px(24.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme.foreground)
                    .child(t!("advanced.title").to_string()),
            )
            .when(!log_match, |this| {
                this.child(
                    theme::tile(cx).child(
                        v_flex()
                            .w_full()
                            .items_center()
                            .justify_center()
                            .py(px(32.))
                            .gap(px(8.))
                            .child(
                                Icon::new(IconName::Search)
                                    .size(px(20.))
                                    .text_color(theme.muted_foreground),
                            )
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(theme.muted_foreground)
                                    .child(t!("advanced.no_matching_advanced").to_string()),
                            ),
                    ),
                )
            })
            .when(log_match, |this| {
                this.child(
                    theme::tile(cx).child(
                        v_flex()
                            .w_full()
                            .gap(px(12.))
                            .child(
                                v_flex()
                                    .gap(px(2.))
                                    .child(theme::tile_label(
                                        t!("advanced.log_config.title").to_string(),
                                        cx,
                                    ))
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(theme.muted_foreground)
                                            .child(
                                                t!("advanced.log_config.description").to_string(),
                                            ),
                                    ),
                            )
                            // 1. 开关: 启用应用诊断日志
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .p(px(8.))
                                    .rounded(px(8.))
                                    .bg(theme.secondary.opacity(0.4))
                                    .child(
                                        h_flex()
                                            .items_center()
                                            .gap(px(10.))
                                            .child(
                                                div()
                                                    .size(px(32.))
                                                    .rounded(px(8.))
                                                    .bg(theme.border)
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .child(
                                                        Icon::new(IconName::Settings2)
                                                            .size(px(16.)),
                                                    ),
                                            )
                                            .child(
                                                v_flex()
                                                    .gap(px(2.))
                                                    .child(
                                                        div()
                                                            .text_size(px(13.))
                                                            .font_weight(FontWeight::MEDIUM)
                                                            .text_color(theme.foreground)
                                                            .child(
                                                                t!("advanced.log_config.enabled")
                                                                    .to_string(),
                                                            ),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_size(px(11.))
                                                            .text_color(theme.muted_foreground)
                                                            .child(
                                                                t!("advanced.log_config.enabled_desc")
                                                                    .to_string(),
                                                            ),
                                                    ),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .id("switch-diagnostic-log")
                                            .cursor_pointer()
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.toggle_log_enabled(window, cx);
                                            }))
                                            .child(self.render_switch(self.log_config.enabled, cx)),
                                    ),
                            )
                            // 2. 日志级别
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .p(px(8.))
                                    .rounded(px(8.))
                                    .bg(theme.secondary.opacity(0.4))
                                    .child(
                                        h_flex()
                                            .items_center()
                                            .gap(px(10.))
                                            .child(
                                                div()
                                                    .size(px(32.))
                                                    .rounded(px(8.))
                                                    .bg(theme.border)
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .child(
                                                        Icon::new(IconName::Settings)
                                                            .size(px(16.)),
                                                    ),
                                            )
                                            .child(
                                                v_flex()
                                                    .gap(px(2.))
                                                    .child(
                                                        div()
                                                            .text_size(px(13.))
                                                            .font_weight(FontWeight::MEDIUM)
                                                            .text_color(theme.foreground)
                                                            .child(
                                                                t!("advanced.log_config.level")
                                                                    .to_string(),
                                                            ),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_size(px(11.))
                                                            .text_color(theme.muted_foreground)
                                                            .child(
                                                                t!("advanced.log_config.level_desc")
                                                                    .to_string(),
                                                            ),
                                                    ),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .w(px(160.))
                                            .child(
                                                Select::new(&self.log_level_select)
                                                    .disabled(!self.log_config.enabled),
                                            ),
                                    ),
                            )
                            // 3. 留存天数
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .p(px(8.))
                                    .rounded(px(8.))
                                    .bg(theme.secondary.opacity(0.4))
                                    .child(
                                        h_flex()
                                            .items_center()
                                            .gap(px(10.))
                                            .child(
                                                div()
                                                    .size(px(32.))
                                                    .rounded(px(8.))
                                                    .bg(theme.border)
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .child(
                                                        Icon::new(IconName::SquareTerminal)
                                                            .size(px(16.)),
                                                    ),
                                            )
                                            .child(
                                                v_flex()
                                                    .gap(px(2.))
                                                    .child(
                                                        div()
                                                            .text_size(px(13.))
                                                            .font_weight(FontWeight::MEDIUM)
                                                            .text_color(theme.foreground)
                                                            .child(
                                                                t!("advanced.log_config.retention")
                                                                    .to_string(),
                                                            ),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_size(px(11.))
                                                            .text_color(theme.muted_foreground)
                                                            .child(
                                                                t!("advanced.log_config.retention_desc")
                                                                    .to_string(),
                                                            ),
                                                    ),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .w(px(160.))
                                            .child(
                                                Select::new(&self.log_retention_select)
                                                    .disabled(!self.log_config.enabled),
                                            ),
                                    ),
                            )
                            // 4. 日志级别说明卡片
                            .child(
                                v_flex()
                                    .w_full()
                                    .p(px(12.))
                                    .rounded(px(8.))
                                    .bg(theme.secondary.opacity(0.5))
                                    .gap(px(6.))
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(theme.foreground)
                                            .child(
                                                t!("advanced.log_config.level_hint").to_string(),
                                            ),
                                    )
                                    .child(
                                        v_flex()
                                            .gap(px(4.))
                                            .text_size(px(11.))
                                            .child(
                                                h_flex()
                                                    .items_center()
                                                    .gap(px(6.))
                                                    .child(
                                                        div()
                                                            .font_weight(FontWeight::SEMIBOLD)
                                                            .text_color(rgb(0xEF4444))
                                                            .child("error"),
                                                    )
                                                    .child(div().text_color(theme.muted_foreground).child("-"))
                                                    .child(
                                                        div()
                                                            .text_color(theme.muted_foreground)
                                                            .child(t!("advanced.log_config.level_desc.error").to_string()),
                                                    ),
                                            )
                                            .child(
                                                h_flex()
                                                    .items_center()
                                                    .gap(px(6.))
                                                    .child(
                                                        div()
                                                            .font_weight(FontWeight::SEMIBOLD)
                                                            .text_color(rgb(0xF59E0B))
                                                            .child("warn"),
                                                    )
                                                    .child(div().text_color(theme.muted_foreground).child("-"))
                                                    .child(
                                                        div()
                                                            .text_color(theme.muted_foreground)
                                                            .child(t!("advanced.log_config.level_desc.warn").to_string()),
                                                    ),
                                            )
                                            .child(
                                                h_flex()
                                                    .items_center()
                                                    .gap(px(6.))
                                                    .child(
                                                        div()
                                                            .font_weight(FontWeight::SEMIBOLD)
                                                            .text_color(rgb(0x3B82F6))
                                                            .child("info"),
                                                    )
                                                    .child(div().text_color(theme.muted_foreground).child("-"))
                                                    .child(
                                                        div()
                                                            .text_color(theme.muted_foreground)
                                                            .child(t!("advanced.log_config.level_desc.info").to_string()),
                                                    ),
                                            )
                                            .child(
                                                h_flex()
                                                    .items_center()
                                                    .gap(px(6.))
                                                    .child(
                                                        div()
                                                            .font_weight(FontWeight::SEMIBOLD)
                                                            .text_color(rgb(0x10B981))
                                                            .child("debug"),
                                                    )
                                                    .child(div().text_color(theme.muted_foreground).child("-"))
                                                    .child(
                                                        div()
                                                            .text_color(theme.muted_foreground)
                                                            .child(t!("advanced.log_config.level_desc.debug").to_string()),
                                                    ),
                                            )
                                            .child(
                                                h_flex()
                                                    .items_center()
                                                    .gap(px(6.))
                                                    .child(
                                                        div()
                                                            .font_weight(FontWeight::SEMIBOLD)
                                                            .text_color(rgb(0x6B7280))
                                                            .child("trace"),
                                                    )
                                                    .child(div().text_color(theme.muted_foreground).child("-"))
                                                    .child(
                                                        div()
                                                            .text_color(theme.muted_foreground)
                                                            .child(t!("advanced.log_config.level_desc.trace").to_string()),
                                                    ),
                                            ),
                                    ),
                            )
                            // 5. 日志路径与操作按钮
                            .child(
                                div()
                                    .p(px(10.))
                                    .rounded(px(8.))
                                    .bg(theme.secondary.opacity(0.4))
                                    .border_1()
                                    .border_color(theme.border)
                                    .child(
                                        h_flex()
                                            .w_full()
                                            .items_center()
                                            .justify_between()
                                            .gap(px(8.))
                                            .child(
                                                v_flex()
                                                    .gap(px(2.))
                                                    .child(
                                                        div()
                                                            .text_size(px(11.))
                                                            .font_weight(FontWeight::MEDIUM)
                                                            .text_color(theme.muted_foreground)
                                                            .child(
                                                                t!("advanced.log_config.path_label")
                                                                    .to_string(),
                                                            ),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_size(px(12.))
                                                            .text_color(theme.foreground)
                                                            .child(
                                                                self.workspace
                                                                    .main_log_path()
                                                                    .to_string_lossy()
                                                                    .to_string(),
                                                            ),
                                                    ),
                                            )
                                            .child(
                                                h_flex()
                                                    .gap(px(8.))
                                                    .child(
                                                        Button::new("btn-open-log-dir")
                                                            .outline()
                                                            .small()
                                                            .icon(IconName::ExternalLink)
                                                            .label(
                                                                t!("advanced.log_config.open_dir")
                                                                    .to_string(),
                                                            )
                                                            .on_click(cx.listener(
                                                                |this, _, window, cx| {
                                                                    this.open_diagnostic_log_dir(
                                                                        window, cx,
                                                                    );
                                                                },
                                                            )),
                                                    )
                                                    .child(
                                                        Button::new("btn-clear-logs")
                                                            .outline()
                                                            .small()
                                                            .icon(IconName::Delete)
                                                            .label(
                                                                t!("advanced.log_config.clear_logs")
                                                                    .to_string(),
                                                            )
                                                            .on_click(cx.listener(
                                                                |this, _, window, cx| {
                                                                    this.clear_diagnostic_logs(
                                                                        window, cx,
                                                                    );
                                                                },
                                                            )),
                                                    ),
                                            ),
                                    ),
                            ),
                    ),
                )
            })
    }

    fn render_about_settings(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

        fn tool_icon_and_color(icon_kind: &str) -> (CustomIcon, Hsla) {
            match icon_kind {
                "claude" => (CustomIcon::Claude, rgb(0xD97757).into()),
                "codex" => (CustomIcon::OpenAI, rgb(0x10A37F).into()),
                "gemini" => (CustomIcon::DeepSeek, rgb(0x3B82F6).into()),
                "grok" => (CustomIcon::Grok, rgb(0x8B5CF6).into()),
                "opencode" => (CustomIcon::OpenCode, rgb(0x0284C7).into()),
                "pi" => (CustomIcon::Pi, rgb(0x3B82F6).into()),
                "openclaw" => (CustomIcon::OhMyPi, rgb(0xEC4899).into()),
                "hermes" => (CustomIcon::Fx, rgb(0x4B5563).into()),
                _ => (CustomIcon::OpenAI, rgb(0x10A37F).into()),
            }
        }

        let upgradable_count = self.env_tools.iter().filter(|t| t.is_upgradable).count();

        v_flex()
            .w_full()
            .gap(px(16.))
            // 1. Top App Info Card
            .child(
                theme::tile(cx)
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .justify_between()
                            .p(px(8.))
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap(px(16.))
                                    .child(
                                        div()
                                            .size(px(52.))
                                            .rounded(px(14.))
                                            .bg(rgb(0x2563EB))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .shadow_md()
                                            .child(
                                                Icon::new(CustomIcon::OpenAI)
                                                    .size(px(28.))
                                                    .text_color(rgb(0xFFFFFF)),
                                            ),
                                    )
                                    .child(
                                        v_flex()
                                            .gap(px(4.))
                                            .child(
                                                h_flex()
                                                    .items_center()
                                                    .gap(px(10.))
                                                    .child(
                                                        div()
                                                            .text_size(px(22.))
                                                            .font_weight(FontWeight::BOLD)
                                                            .text_color(theme.foreground)
                                                            .child("AICWITCH"),
                                                    )
                                                    .child(
                                                        Tag::primary()
                                                            .small()
                                                            .child(format!("v{}", env!("CARGO_PKG_VERSION"))),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(12.))
                                                    .text_color(theme.muted_foreground)
                                                    .child(t!("app.subtitle").to_string()),
                                            ),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .child(
                                        Button::new("about-preview-update")
                                            .outline()
                                            .small()
                                            .icon(IconName::Bell)
                                            .label(t!("update.preview_dialog"))
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.preview_update_dialog(window, cx);
                                            })),
                                    )
                                    .child(
                                        Button::new("about-github")
                                            .outline()
                                            .small()
                                            .icon(IconName::GitHub)
                                            .label("GitHub")
                                            .on_click(cx.listener(|_, _, window, cx| {
                                                crate::update_dialog::open_url("https://github.com/aohun/router-switch");
                                                window.push_notification(Notification::info("https://github.com/aohun/router-switch"), cx);
                                            })),
                                    )
                                    .child(
                                        Button::new("about-changelog")
                                            .outline()
                                            .small()
                                            .icon(IconName::File)
                                            .label(t!("about.changelog"))
                                            .on_click(cx.listener(|_, _, _window, _cx| {
                                                crate::update_dialog::open_url("https://github.com/aohun/router-switch/releases");
                                            })),
                                    )
                                    .child(
                                        Button::new("about-check-update")
                                            .primary()
                                            .small()
                                            .disabled(self.is_checking_update)
                                            .icon(CustomIcon::RotateCw)
                                            .label(if self.is_checking_update {
                                                t!("update.checking_update")
                                            } else {
                                                t!("about.check_update")
                                            })
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.check_for_updates(true, window, cx);
                                            })),
                                    ),
                            ),
                    ),
            )
            // 2. Local Environment Check Section
            .child(
                theme::tile(cx).child(
                    v_flex()
                        .w_full()
                        .gap(px(14.))
                        // Section Header Bar
                        .child(
                            h_flex()
                                .w_full()
                                .items_center()
                                .justify_between()
                                .child(
                                    v_flex()
                                        .gap(px(2.))
                                        .child(theme::tile_label(t!("about.env_check_title").to_string(), cx))
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .text_color(theme.muted_foreground)
                                                .child(t!("about.env_check_desc").to_string()),
                                        ),
                                )
                                .child(
                                    h_flex()
                                        .items_center()
                                        .gap(px(8.))
                                        .child(
                                            Button::new("env-diagnose-btn")
                                                .outline()
                                                .small()
                                                .icon(IconName::TriangleAlert)
                                                .label(t!("about.diagnose_conflicts"))
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    this.diagnose_all_conflicts(window, cx);
                                                })),
                                        )
                                        .child(
                                            Button::new("env-refresh-btn")
                                                .outline()
                                                .small()
                                                .icon(CustomIcon::RotateCw)
                                                .label(t!("about.refresh"))
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    this.refresh_env(window, cx);
                                                })),
                                        )
                                        .child(
                                            Button::new("env-upgrade-all-btn")
                                                .primary()
                                                .small()
                                                .icon(IconName::ArrowUp)
                                                .label(format!("全部升级 ({})", upgradable_count))
                                                .disabled(upgradable_count == 0)
                                                .on_click(cx.listener(move |this, _, window, cx| {
                                                    let upgradable_ids: Vec<String> = this
                                                        .env_tools
                                                        .iter()
                                                        .filter(|t| t.is_upgradable)
                                                        .map(|t| t.id.clone())
                                                        .collect();
                                                    for tid in upgradable_ids {
                                                        this.run_tool_upgrade(&tid, window, cx);
                                                    }
                                                })),
                                        ),
                                ),
                        )
                        // 2-Column CLI Tool Grid
                        .child(
                            if self.env_tools.is_empty() {
                                if self.is_inspecting_env {
                                    v_flex()
                                        .w_full()
                                        .items_center()
                                        .justify_center()
                                        .py(px(40.))
                                        .gap(px(10.))
                                        .child(Icon::new(CustomIcon::RotateCw).size(px(24.)).text_color(theme.primary))
                                        .child(
                                            div()
                                                .text_size(px(13.))
                                                .text_color(theme.muted_foreground)
                                                .child(t!("about.inspecting_tip").to_string()),
                                        )
                                        .into_any_element()
                                } else {
                                    v_flex()
                                        .w_full()
                                        .items_center()
                                        .justify_center()
                                        .py(px(32.))
                                        .gap(px(12.))
                                        .child(Icon::new(IconName::SquareTerminal).size(px(32.)).text_color(theme.muted_foreground))
                                        .child(
                                            div()
                                                .text_size(px(13.))
                                                .text_color(theme.muted_foreground)
                                                .child(t!("about.empty_inspect_tip").to_string()),
                                        )
                                        .child(
                                            Button::new("about-start-inspect-btn")
                                                .primary()
                                                .icon(CustomIcon::RotateCw)
                                                .label(t!("about.start_inspect"))
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    this.refresh_env(window, cx);
                                                })),
                                        )
                                        .into_any_element()
                                }
                            } else {
                                v_flex()
                                    .w_full()
                                    .gap(px(10.))
                                    .children(self.env_tools.chunks(2).map(|pair| {
                                        h_flex()
                                            .w_full()
                                            .gap(px(10.))
                                            .children(pair.iter().map(|tool| {
                                                let (icon, icon_color) = tool_icon_and_color(&tool.icon_kind);
                                                let tool_id = tool.id.clone();
                                                let tool_name = tool.name.clone();
                                                let is_up = tool.is_upgradable;
                                                let is_inst = tool.is_installed;
                                                let is_broken = tool.installed_but_broken;
                                                let has_conflict = tool.has_conflicts;

                                                let current_ver_display = tool
                                                    .current_version
                                                    .clone()
                                                    .unwrap_or_else(|| if is_broken { "无法运行".into() } else { "未安装".into() });

                                                let latest_ver_display = tool
                                                    .latest_version
                                                    .clone()
                                                    .unwrap_or_else(|| if self.is_inspecting_env { "检测中...".into() } else { "未知".into() });

                                                let status_tag_text = if is_up {
                                                    "可升级"
                                                } else if is_broken {
                                                    "无法运行"
                                                } else if is_inst {
                                                    "已就绪"
                                                } else {
                                                    "未安装"
                                                };

                                                div()
                                                    .flex_1()
                                                    .p(px(14.))
                                                    .rounded(px(10.))
                                                    .bg(theme.secondary.opacity(0.35))
                                                    .border_1()
                                                    .border_color(if has_conflict { Hsla::from(rgb(0xF59E0B)).opacity(0.4) } else { theme.border })
                                                    .gap(px(10.))
                                                    .child(
                                                        // Card Header
                                                        h_flex()
                                                            .w_full()
                                                            .items_center()
                                                            .justify_between()
                                                            .child(
                                                                h_flex()
                                                                    .items_center()
                                                                    .gap(px(10.))
                                                                    .child(
                                                                        div()
                                                                            .size(px(36.))
                                                                            .rounded(px(8.))
                                                                            .bg(theme.border.opacity(0.5))
                                                                            .flex()
                                                                            .items_center()
                                                                            .justify_center()
                                                                            .child(Icon::new(icon).size(px(18.)).text_color(icon_color)),
                                                                    )
                                                                    .child(
                                                                        v_flex()
                                                                            .gap(px(2.))
                                                                            .child(
                                                                                h_flex()
                                                                                    .items_center()
                                                                                    .gap(px(6.))
                                                                                    .child(
                                                                                        div()
                                                                                            .text_size(px(14.))
                                                                                        .font_weight(FontWeight::SEMIBOLD)
                                                                                        .text_color(theme.foreground)
                                                                                        .child(tool_name.clone()),
                                                                                )
                                                                                .child(
                                                                                    div()
                                                                                        .px(px(5.))
                                                                                        .py(px(0.5))
                                                                                        .rounded(px(4.))
                                                                                        .bg(theme.border)
                                                                                        .text_size(px(10.))
                                                                                        .text_color(theme.muted_foreground)
                                                                                        .child(tool.platform.clone()),
                                                                                ),
                                                                        ),
                                                                ),
                                                        )
                                                        .child(
                                                            if is_up {
                                                                Tag::primary().small().child(status_tag_text)
                                                            } else if is_broken {
                                                                Tag::danger().small().child(status_tag_text)
                                                            } else {
                                                                Tag::secondary().small().child(status_tag_text)
                                                            }
                                                        ),
                                                )
                                                .child(
                                                    // Version details rows
                                                    v_flex()
                                                        .w_full()
                                                        .gap(px(4.))
                                                        .child(
                                                            h_flex()
                                                                .w_full()
                                                                .items_center()
                                                                .justify_between()
                                                                .text_size(px(12.))
                                                                .child(div().text_color(theme.muted_foreground).child(t!("about.current_version").to_string()))
                                                                .child(div().font_weight(FontWeight::MEDIUM).text_color(theme.foreground).child(current_ver_display)),
                                                        )
                                                        .child(
                                                            h_flex()
                                                                .w_full()
                                                                .items_center()
                                                                .justify_between()
                                                                .text_size(px(12.))
                                                                .child(div().text_color(theme.muted_foreground).child(t!("about.latest_version").to_string()))
                                                                .child(div().font_weight(FontWeight::MEDIUM).text_color(theme.foreground).child(latest_ver_display)),
                                                        ),
                                                )
                                                .when(has_conflict, |this| {
                                                    this.child(
                                                        v_flex()
                                                            .w_full()
                                                            .p(px(8.))
                                                            .rounded(px(6.))
                                                            .bg(rgba(0xF59E0B12))
                                                            .border_1()
                                                            .border_color(rgba(0xF59E0B44))
                                                            .gap(px(4.))
                                                            .child(
                                                                h_flex()
                                                                    .items_center()
                                                                    .gap(px(4.))
                                                                    .child(Icon::new(IconName::TriangleAlert).size(px(12.)).text_color(rgb(0xD97706)))
                                                                    .child(
                                                                        div()
                                                                            .text_size(px(11.))
                                                                            .font_weight(FontWeight::SEMIBOLD)
                                                                            .text_color(rgb(0xD97706))
                                                                            .child(t!("about.multiple_installations").to_string()),
                                                                    ),
                                                            )
                                                            .child(
                                                                div()
                                                                    .text_size(px(10.))
                                                                    .text_color(theme.muted_foreground)
                                                                    .child(t!("about.multiple_installations_tip").to_string()),
                                                            )
                                                            .children(tool.installations.iter().map(|inst| {
                                                                h_flex()
                                                                    .w_full()
                                                                    .items_center()
                                                                    .justify_between()
                                                                    .gap(px(6.))
                                                                    .text_size(px(10.))
                                                                    .child(
                                                                        h_flex()
                                                                            .items_center()
                                                                            .gap(px(4.))
                                                                            .flex_1()
                                                                            .child(
                                                                                div()
                                                                                    .px(px(4.))
                                                                                    .py(px(0.5))
                                                                                    .rounded(px(3.))
                                                                                    .bg(theme.border)
                                                                                    .font_weight(FontWeight::MEDIUM)
                                                                                    .text_color(theme.muted_foreground)
                                                                                    .child(inst.source.clone()),
                                                                            )
                                                                            .child(
                                                                                div()
                                                                                    .line_clamp(1)
                                                                                    .text_color(theme.muted_foreground)
                                                                                    .child(inst.path.clone()),
                                                                            ),
                                                                    )
                                                                    .child(
                                                                        h_flex()
                                                                            .items_center()
                                                                            .gap(px(4.))
                                                                            .child(
                                                                                div()
                                                                                    .font_weight(FontWeight::MEDIUM)
                                                                                    .text_color(if inst.runnable { theme.foreground } else { rgb(0xDC2626).into() })
                                                                                    .child(if inst.runnable { inst.version.clone().unwrap_or_default() } else { "无法运行".into() }),
                                                                            )
                                                                            .when(inst.is_path_default, |this| {
                                                                                this.child(
                                                                                    div()
                                                                                        .px(px(4.))
                                                                                        .py(px(0.5))
                                                                                        .rounded_full()
                                                                                        .bg(theme.primary.opacity(0.15))
                                                                                        .text_color(theme.primary)
                                                                                        .font_weight(FontWeight::SEMIBOLD)
                                                                                        .child(t!("about.default_badge").to_string()),
                                                                                )
                                                                            }),
                                                                    )
                                                            }))
                                                    )
                                                })
                                                .child(
                                                    // Bottom Action Button
                                                    h_flex()
                                                        .w_full()
                                                        .justify_end()
                                                        .child(
                                                            if is_up {
                                                                let tid = tool_id.clone();
                                                                Button::new(SharedString::from(format!("tool-up-{}", tool_id)))
                                                                    .primary()
                                                                    .small()
                                                                    .icon(IconName::ArrowUp)
                                                                    .label(t!("about.upgrade"))
                                                                    .on_click(cx.listener(move |this, _, window, cx| {
                                                                        this.run_tool_upgrade(&tid, window, cx);
                                                                    }))
                                                                    .into_any_element()
                                                            } else if is_broken {
                                                                div()
                                                                    .text_size(px(11.))
                                                                    .text_color(rgb(0xD97706))
                                                                    .child("请检查运行依赖")
                                                                    .into_any_element()
                                                            } else if is_inst {
                                                                Button::new(SharedString::from(format!("tool-chk-{}", tool_id)))
                                                                    .outline()
                                                                    .small()
                                                                    .disabled(true)
                                                                    .label(t!("about.uptodate"))
                                                                    .into_any_element()
                                                            } else {
                                                                let tid = tool_id.clone();
                                                                Button::new(SharedString::from(format!("tool-ins-{}", tool_id)))
                                                                    .outline()
                                                                    .small()
                                                                    .icon(IconName::Plus)
                                                                    .label(t!("about.install"))
                                                                    .on_click(cx.listener(move |this, _, window, cx| {
                                                                        this.run_tool_install(&tid, window, cx);
                                                                    }))
                                                                    .into_any_element()
                                                            }
                                                        )
                                                )
                                        }))
                                        .when(pair.len() == 1, |this| this.child(div().flex_1()))
                                    }))
                                    .into_any_element()
                            }
                        ),
                ),
            )
    }

    fn render_settings_page(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(match self.settings_tab {
            SettingsTab::General => self.render_general_settings(cx).into_any_element(),
            SettingsTab::Advanced => self.render_advanced_settings(cx).into_any_element(),
            SettingsTab::About => self.render_about_settings(cx).into_any_element(),
        })
    }

    fn render_notifications_page(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap(px(16.))
            .child(
                div()
                    .text_size(px(28.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(cx.theme().foreground)
                    .child("Notifications"),
            )
            .child(
                theme::tile(cx).child(
                    v_flex()
                        .w_full()
                        .gap(px(8.))
                        .child(theme::tile_label("ACTIVITY LOG / 操作日志", cx))
                        .children(self.logs.iter().rev().map(|log| {
                            h_flex()
                                .w_full()
                                .items_center()
                                .gap(px(8.))
                                .py(px(4.))
                                .child(div().size(px(6.)).rounded_full().bg(cx.theme().primary))
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .text_color(cx.theme().foreground)
                                        .child(log.clone()),
                                )
                        })),
                ),
            )
    }

    fn render_form_page(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(form) = self.form.as_ref() else {
            return div().into_any_element();
        };

        let app_name = form.app.display_name();
        let is_editing = form.editing_id.is_some();
        let title = if is_editing {
            format!("编辑 {} 服务商", app_name)
        } else {
            format!("新建 {} 服务商", app_name)
        };
        let subtitle = if is_editing {
            "修改服务商的接口端点、模型名称与模型映射配置"
        } else {
            "从预设模版快速创建或手动填写第三方 API 服务商"
        };
        let theme = cx.theme();

        v_flex()
            .w_full()
            .gap(px(16.))
            .child(
                // Header bar with breadcrumbs / back button and actions
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .child(
                        h_flex()
                            .items_center()
                            .gap(px(8.))
                            .child(
                                Button::new("back-btn")
                                    .ghost()
                                    .small()
                                    .icon(IconName::ChevronLeft)
                                    .label("返回")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.form = None;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(theme.muted_foreground)
                                    .child("/"),
                            )
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(theme.muted_foreground)
                                    .child(format!("{} 服务商", app_name)),
                            )
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(theme.muted_foreground)
                                    .child("/"),
                            )
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(theme.foreground)
                                    .child(title.clone()),
                            ),
                    )
                    .child(
                        h_flex()
                            .items_center()
                            .gap(px(8.))
                            .child(
                                Button::new("clipboard-import-header-btn")
                                    .outline()
                                    .small()
                                    .icon(IconName::Copy)
                                    .label(t!("provider.clipboard_import").to_string())
                                    .tooltip(t!("provider.clipboard_import_tip").to_string())
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.import_from_clipboard(window, cx);
                                    })),
                            )
                            .child(
                                Button::new("cancel-page-btn")
                                    .outline()
                                    .small()
                                    .label(t!("provider.cancel").to_string())
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.form = None;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("save-page-btn")
                                    .primary()
                                    .small()
                                    .icon(IconName::Check)
                                    .label(t!("provider.save").to_string())
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.submit_form(window, cx);
                                    })),
                            ),
                    ),
            )
            .child(
                v_flex()
                    .gap(px(2.))
                    .child(
                        div()
                            .text_size(px(22.))
                            .font_weight(FontWeight::BOLD)
                            .text_color(theme.foreground)
                            .child(title),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(theme.muted_foreground)
                            .child(subtitle),
                    ),
            )
            .child(
                // Preset Template Selection Card
                theme::tile(cx).child(
                    v_flex()
                        .w_full()
                        .gap(px(10.))
                        .child(theme::tile_label("PRESET TEMPLATE / 快速选择预设模版", cx))
                        .child(
                            Select::new(&form.preset_select)
                                .placeholder("选择预设模版...")
                                .search_placeholder("搜索预设模版...")
                                .cleanable(true),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(theme.muted_foreground)
                                .child("💡 提示：选择预设模版会自动为您填入官方端点及推荐模型。"),
                        ),
                ),
            )
            .child(
                // Basic & API Credentials Card
                theme::tile(cx).child(
                    v_flex()
                        .w_full()
                        .gap(px(12.))
                        .child(
                            h_flex()
                                .w_full()
                                .items_center()
                                .justify_between()
                                .child(theme::tile_label("BASIC & API CREDENTIALS / 基础配置与接口凭证", cx))
                                .child(
                                    Button::new("clipboard-import-card-btn")
                                        .ghost()
                                        .small()
                                        .icon(IconName::Copy)
                                        .label(t!("provider.clipboard_import").to_string())
                                        .tooltip(t!("provider.clipboard_import_tip").to_string())
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.import_from_clipboard(window, cx);
                                        })),
                                ),
                        )
                        .child(form_field("服务商名称", Input::new(&form.name)))
                        .child(form_field(
                            "API Key / 凭据",
                            Input::new(&form.api_key).mask_toggle(),
                        ))
                                                .child(
                            v_flex()
                                .gap(px(6.))
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(theme.foreground)
                                        .child("API 端点 (Base URL)"),
                                )
                                .child(
                                    h_flex()
                                        .w_full()
                                        .gap(px(8.))
                                        .items_center()
                                        .child(
                                            div().flex_1().child(Input::new(&form.base_url))
                                        )
                                        .child(
                                            Button::new("test-form-connectivity-btn")
                                                .outline()
                                                .icon(CustomIcon::Activity)
                                                .label(if form.is_testing_connectivity {
                                                    t!("provider.testing_connectivity").to_string()
                                                } else {
                                                    t!("provider.test_connectivity").to_string()
                                                })
                                                .tooltip("测试当前 API 端点网络连通性")
                                                .disabled(form.is_testing_connectivity)
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    this.test_form_connectivity(window, cx);
                                                })),
                                        ),
                                )
                                .when_some(form.connectivity_result.as_ref(), |this, res| {
                                    let (tag_text, is_success, is_warn) = match res.status {
                                        domain::HealthStatus::Operational => (
                                            format!(
                                                "🟢 {} ({}ms) - {}",
                                                t!("provider.connectivity_operational"),
                                                res.latency_ms.unwrap_or(0),
                                                res.message
                                            ),
                                            true,
                                            false,
                                        ),
                                        domain::HealthStatus::Degraded => (
                                            format!(
                                                "🟡 {} ({}ms) - {}",
                                                t!("provider.connectivity_degraded"),
                                                res.latency_ms.unwrap_or(0),
                                                res.message
                                            ),
                                            false,
                                            true,
                                        ),
                                        domain::HealthStatus::Failed => (
                                            format!(
                                                "🔴 {}: {}",
                                                t!("provider.connectivity_failed"),
                                                res.message
                                            ),
                                            false,
                                            false,
                                        ),
                                    };
                                    let tag = if is_success {
                                        Tag::success().small().child(tag_text)
                                    } else if is_warn {
                                        Tag::warning().small().child(tag_text)
                                    } else {
                                        Tag::danger().small().child(tag_text)
                                    };
                                    this.child(div().pt(px(2.)).child(tag))
                                }),
                        )
                        .child(
                            v_flex()
                                .gap(px(6.))
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(theme.foreground)
                                        .child("默认模型 (Model)"),
                                )
                                .child(
                                    h_flex()
                                        .w_full()
                                        .gap(px(8.))
                                        .items_center()
                                        .child(
                                            div().flex_1().child(Input::new(&form.model).cleanable(true))
                                        )
                                        .when_some(form.default_model_select.as_ref(), |this, select| {
                                            this.child(
                                                div()
                                                    .w(px(240.))
                                                    .child(
                                                        Select::new(select)
                                                            .placeholder("选择模型...")
                                                            .search_placeholder("搜索模型..."),
                                                    ),
                                            )
                                        })
                                        .child(
                                            Button::new("fetch-models-btn")
                                                .outline()
                                                .icon(IconName::ArrowDown)
                                                .tooltip("从端点拉取可用模型列表")
                                                .disabled(form.is_fetching_models)
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    this.fetch_models_for_form(window, cx);
                                                })),
                                        ),
                                ),
                        )
                        .when(form.app == AppKind::WorkBuddy, |this| {
                            let tool_checked = form.workbuddy_supports_tool_call;
                            let image_checked = form.workbuddy_supports_images;
                            let reason_checked = form.workbuddy_supports_reasoning;
                            let custom_proto_checked = form.workbuddy_use_custom_protocol;
                            let low_checked = form.workbuddy_supported_effort_low;
                            let med_checked = form.workbuddy_supported_effort_medium;
                            let high_checked = form.workbuddy_supported_effort_high;
                            let xhigh_checked = form.workbuddy_supported_effort_xhigh;
                            let max_checked = form.workbuddy_supported_effort_max;
                            let reasoning_only = form.workbuddy_reasoning_only;
                            let can_disable = form.workbuddy_can_disable_reasoning;

                            this.child(
                                v_flex()
                                    .gap(px(6.))
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(theme.foreground)
                                            .child("特性支持 (Capabilities)"),
                                    )
                                    .child(
                                        h_flex()
                                            .gap(px(16.))
                                            .items_center()
                                            .flex_wrap()
                                            .child(
                                                h_flex()
                                                    .id("checkbox-wb-tool")
                                                    .gap(px(6.))
                                                    .items_center()
                                                    .cursor_pointer()
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.toggle_workbuddy_supports_tool_call(cx);
                                                    }))
                                                    .child(
                                                        div()
                                                            .size(px(18.))
                                                            .rounded(px(4.))
                                                            .border_1()
                                                            .border_color(if tool_checked { theme.primary } else { theme.border })
                                                            .bg(if tool_checked { theme.primary } else { gpui::transparent_black() })
                                                            .flex()
                                                            .items_center()
                                                            .justify_center()
                                                            .when(tool_checked, |this| {
                                                                this.child(
                                                                    Icon::new(IconName::Check)
                                                                        .size(px(13.))
                                                                        .text_color(rgb(0xFFFFFF)),
                                                                )
                                                            }),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_size(px(13.))
                                                            .text_color(theme.foreground)
                                                            .child("函数调用 (Tool Call)"),
                                                    ),
                                            )
                                            .child(
                                                h_flex()
                                                    .id("checkbox-wb-image")
                                                    .gap(px(6.))
                                                    .items_center()
                                                    .cursor_pointer()
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.toggle_workbuddy_supports_images(cx);
                                                    }))
                                                    .child(
                                                        div()
                                                            .size(px(18.))
                                                            .rounded(px(4.))
                                                            .border_1()
                                                            .border_color(if image_checked { theme.primary } else { theme.border })
                                                            .bg(if image_checked { theme.primary } else { gpui::transparent_black() })
                                                            .flex()
                                                            .items_center()
                                                            .justify_center()
                                                            .when(image_checked, |this| {
                                                                this.child(
                                                                    Icon::new(IconName::Check)
                                                                        .size(px(13.))
                                                                        .text_color(rgb(0xFFFFFF)),
                                                                )
                                                            }),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_size(px(13.))
                                                            .text_color(theme.foreground)
                                                            .child("图片输入 (Images)"),
                                                    ),
                                            )
                                            .child(
                                                h_flex()
                                                    .id("checkbox-wb-reason")
                                                    .gap(px(6.))
                                                    .items_center()
                                                    .cursor_pointer()
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.toggle_workbuddy_supports_reasoning(cx);
                                                    }))
                                                    .child(
                                                        div()
                                                            .size(px(18.))
                                                            .rounded(px(4.))
                                                            .border_1()
                                                            .border_color(if reason_checked { theme.primary } else { theme.border })
                                                            .bg(if reason_checked { theme.primary } else { gpui::transparent_black() })
                                                            .flex()
                                                            .items_center()
                                                            .justify_center()
                                                            .when(reason_checked, |this| {
                                                                this.child(
                                                                    Icon::new(IconName::Check)
                                                                        .size(px(13.))
                                                                        .text_color(rgb(0xFFFFFF)),
                                                                )
                                                            }),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_size(px(13.))
                                                            .text_color(theme.foreground)
                                                            .child("深度思考 (Reasoning)"),
                                                    ),
                                            )
                                            .child(
                                                h_flex()
                                                    .id("checkbox-wb-custom-proto")
                                                    .gap(px(6.))
                                                    .items_center()
                                                    .cursor_pointer()
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.toggle_workbuddy_use_custom_protocol(cx);
                                                    }))
                                                    .child(
                                                        div()
                                                            .size(px(18.))
                                                            .rounded(px(4.))
                                                            .border_1()
                                                            .border_color(if custom_proto_checked { theme.primary } else { theme.border })
                                                            .bg(if custom_proto_checked { theme.primary } else { gpui::transparent_black() })
                                                            .flex()
                                                            .items_center()
                                                            .justify_center()
                                                            .when(custom_proto_checked, |this| {
                                                                this.child(
                                                                    Icon::new(IconName::Check)
                                                                        .size(px(13.))
                                                                        .text_color(rgb(0xFFFFFF)),
                                                                )
                                                            }),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_size(px(13.))
                                                            .text_color(theme.foreground)
                                                            .child("自定义协议 (Custom Protocol)"),
                                                    ),
                                            ),
                                    ),
                            )
                            .when(reason_checked, |this| {
                                this.child(
                                    v_flex()
                                        .gap(px(8.))
                                        .p(px(12.))
                                        .rounded(px(8.))
                                        .bg(theme.secondary.opacity(0.3))
                                        .border_1()
                                        .border_color(theme.border)
                                        .child(
                                            h_flex()
                                                .w_full()
                                                .gap(px(12.))
                                                .items_center()
                                                .child(
                                                    div()
                                                        .flex_1()
                                                        .child(
                                                            v_flex()
                                                                .gap(px(4.))
                                                                .child(
                                                                    div()
                                                                        .text_size(px(12.))
                                                                        .font_weight(FontWeight::MEDIUM)
                                                                        .text_color(theme.foreground)
                                                                        .child("默认思考强度 (Default Effort)"),
                                                                )
                                                                .when_some(form.workbuddy_reasoning_effort_select.as_ref(), |this, select| {
                                                                    this.child(
                                                                        Select::new(select)
                                                                            .placeholder("选择默认思考强度..."),
                                                                    )
                                                                }),
                                                        ),
                                                ),
                                        )
                                        .child(
                                            v_flex()
                                                .gap(px(4.))
                                                .child(
                                                    div()
                                                        .text_size(px(12.))
                                                        .font_weight(FontWeight::MEDIUM)
                                                        .text_color(theme.foreground)
                                                        .child("支持的思考强度档位 (Supported Efforts)"),
                                                )
                                                .child(
                                                    h_flex()
                                                        .gap(px(12.))
                                                        .items_center()
                                                        .flex_wrap()
                                                        .child(
                                                            h_flex()
                                                                .id("wb-effort-low")
                                                                .gap(px(6.))
                                                                .items_center()
                                                                .cursor_pointer()
                                                                .on_click(cx.listener(|this, _, _, cx| {
                                                                    this.toggle_workbuddy_supported_effort("low", cx);
                                                                }))
                                                                .child(
                                                                    div()
                                                                        .size(px(18.))
                                                                        .rounded(px(4.))
                                                                        .border_1()
                                                                        .border_color(if low_checked { theme.primary } else { theme.border })
                                                                        .bg(if low_checked { theme.primary } else { gpui::transparent_black() })
                                                                        .flex()
                                                                        .items_center()
                                                                        .justify_center()
                                                                        .when(low_checked, |this| {
                                                                            this.child(
                                                                                Icon::new(IconName::Check)
                                                                                    .size(px(13.))
                                                                                    .text_color(rgb(0xFFFFFF)),
                                                                            )
                                                                        }),
                                                                )
                                                                .child(
                                                                    div()
                                                                        .text_size(px(13.))
                                                                        .text_color(theme.foreground)
                                                                        .child("Low (低)"),
                                                                ),
                                                        )
                                                        .child(
                                                            h_flex()
                                                                .id("wb-effort-medium")
                                                                .gap(px(6.))
                                                                .items_center()
                                                                .cursor_pointer()
                                                                .on_click(cx.listener(|this, _, _, cx| {
                                                                    this.toggle_workbuddy_supported_effort("medium", cx);
                                                                }))
                                                                .child(
                                                                    div()
                                                                        .size(px(18.))
                                                                        .rounded(px(4.))
                                                                        .border_1()
                                                                        .border_color(if med_checked { theme.primary } else { theme.border })
                                                                        .bg(if med_checked { theme.primary } else { gpui::transparent_black() })
                                                                        .flex()
                                                                        .items_center()
                                                                        .justify_center()
                                                                        .when(med_checked, |this| {
                                                                            this.child(
                                                                                Icon::new(IconName::Check)
                                                                                    .size(px(13.))
                                                                                    .text_color(rgb(0xFFFFFF)),
                                                                            )
                                                                        }),
                                                                )
                                                                .child(
                                                                    div()
                                                                        .text_size(px(13.))
                                                                        .text_color(theme.foreground)
                                                                        .child("Medium (中)"),
                                                                ),
                                                        )
                                                        .child(
                                                            h_flex()
                                                                .id("wb-effort-high")
                                                                .gap(px(6.))
                                                                .items_center()
                                                                .cursor_pointer()
                                                                .on_click(cx.listener(|this, _, _, cx| {
                                                                    this.toggle_workbuddy_supported_effort("high", cx);
                                                                }))
                                                                .child(
                                                                    div()
                                                                        .size(px(18.))
                                                                        .rounded(px(4.))
                                                                        .border_1()
                                                                        .border_color(if high_checked { theme.primary } else { theme.border })
                                                                        .bg(if high_checked { theme.primary } else { gpui::transparent_black() })
                                                                        .flex()
                                                                        .items_center()
                                                                        .justify_center()
                                                                        .when(high_checked, |this| {
                                                                            this.child(
                                                                                Icon::new(IconName::Check)
                                                                                    .size(px(13.))
                                                                                    .text_color(rgb(0xFFFFFF)),
                                                                            )
                                                                        }),
                                                                )
                                                                .child(
                                                                    div()
                                                                        .text_size(px(13.))
                                                                        .text_color(theme.foreground)
                                                                        .child("High (高)"),
                                                                ),
                                                        )
                                                        .child(
                                                            h_flex()
                                                                .id("wb-effort-xhigh")
                                                                .gap(px(6.))
                                                                .items_center()
                                                                .cursor_pointer()
                                                                .on_click(cx.listener(|this, _, _, cx| {
                                                                    this.toggle_workbuddy_supported_effort("xhigh", cx);
                                                                }))
                                                                .child(
                                                                    div()
                                                                        .size(px(18.))
                                                                        .rounded(px(4.))
                                                                        .border_1()
                                                                        .border_color(if xhigh_checked { theme.primary } else { theme.border })
                                                                        .bg(if xhigh_checked { theme.primary } else { gpui::transparent_black() })
                                                                        .flex()
                                                                        .items_center()
                                                                        .justify_center()
                                                                        .when(xhigh_checked, |this| {
                                                                            this.child(
                                                                                Icon::new(IconName::Check)
                                                                                    .size(px(13.))
                                                                                    .text_color(rgb(0xFFFFFF)),
                                                                            )
                                                                        }),
                                                                )
                                                                .child(
                                                                    div()
                                                                        .text_size(px(13.))
                                                                        .text_color(theme.foreground)
                                                                        .child("Extra High (极高)"),
                                                                ),
                                                        )
                                                        .child(
                                                            h_flex()
                                                                .id("wb-effort-max")
                                                                .gap(px(6.))
                                                                .items_center()
                                                                .cursor_pointer()
                                                                .on_click(cx.listener(|this, _, _, cx| {
                                                                    this.toggle_workbuddy_supported_effort("max", cx);
                                                                }))
                                                                .child(
                                                                    div()
                                                                        .size(px(18.))
                                                                        .rounded(px(4.))
                                                                        .border_1()
                                                                        .border_color(if max_checked { theme.primary } else { theme.border })
                                                                        .bg(if max_checked { theme.primary } else { gpui::transparent_black() })
                                                                        .flex()
                                                                        .items_center()
                                                                        .justify_center()
                                                                        .when(max_checked, |this| {
                                                                            this.child(
                                                                                Icon::new(IconName::Check)
                                                                                    .size(px(13.))
                                                                                    .text_color(rgb(0xFFFFFF)),
                                                                            )
                                                                        }),
                                                                )
                                                                .child(
                                                                    div()
                                                                        .text_size(px(13.))
                                                                        .text_color(theme.foreground)
                                                                        .child("Max (最大)"),
                                                                ),
                                                        ),
                                                ),
                                        )
                                        .child(
                                            h_flex()
                                                .gap(px(16.))
                                                .items_center()
                                                .child(
                                                    h_flex()
                                                        .id("wb-reasoning-only")
                                                        .gap(px(6.))
                                                        .items_center()
                                                        .cursor_pointer()
                                                        .on_click(cx.listener(|this, _, _, cx| {
                                                            this.toggle_workbuddy_reasoning_only(cx);
                                                        }))
                                                        .child(
                                                            div()
                                                                .size(px(18.))
                                                                .rounded(px(4.))
                                                                .border_1()
                                                                .border_color(if reasoning_only { theme.primary } else { theme.border })
                                                                .bg(if reasoning_only { theme.primary } else { gpui::transparent_black() })
                                                                .flex()
                                                                .items_center()
                                                                .justify_center()
                                                                .when(reasoning_only, |this| {
                                                                    this.child(
                                                                        Icon::new(IconName::Check)
                                                                            .size(px(13.))
                                                                            .text_color(rgb(0xFFFFFF)),
                                                                    )
                                                                }),
                                                        )
                                                        .child(
                                                            div()
                                                                .text_size(px(13.))
                                                                .text_color(theme.foreground)
                                                                .child("仅允许思考 (reasoning_only)"),
                                                        ),
                                                )
                                                .child(
                                                    h_flex()
                                                        .id("wb-can-disable-reasoning")
                                                        .gap(px(6.))
                                                        .items_center()
                                                        .cursor_pointer()
                                                        .on_click(cx.listener(|this, _, _, cx| {
                                                            this.toggle_workbuddy_can_disable_reasoning(cx);
                                                        }))
                                                        .child(
                                                            div()
                                                                .size(px(18.))
                                                                .rounded(px(4.))
                                                                .border_1()
                                                                .border_color(if can_disable { theme.primary } else { theme.border })
                                                                .bg(if can_disable { theme.primary } else { gpui::transparent_black() })
                                                                .flex()
                                                                .items_center()
                                                                .justify_center()
                                                                .when(can_disable, |this| {
                                                                    this.child(
                                                                        Icon::new(IconName::Check)
                                                                            .size(px(13.))
                                                                            .text_color(rgb(0xFFFFFF)),
                                                                    )
                                                                }),
                                                        )
                                                        .child(
                                                            div()
                                                                .text_size(px(13.))
                                                                .text_color(theme.foreground)
                                                                .child("允许关闭思考 (can_disable_reasoning)"),
                                                        ),
                                                ),
                                        ),
                                )
                            })
                            .child(
                                h_flex()
                                    .w_full()
                                    .gap(px(12.))
                                    .child(
                                        div()
                                            .flex_1()
                                            .child(form_field("最大输入 Tokens", Input::new(&form.workbuddy_max_input_tokens))),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .child(form_field("最大输出 Tokens", Input::new(&form.workbuddy_max_output_tokens))),
                                    ),
                            )
                        })
                        .when(form.app == AppKind::ZCode, |this| {
                            let text_checked = form.zcode_modality_text;
                            let image_checked = form.zcode_modality_image;
                            this.child(
                                v_flex()
                                    .gap(px(6.))
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(theme.foreground)
                                            .child(t!("provider.modalities").to_string()),
                                    )
                                    .child(
                                        h_flex()
                                            .gap(px(16.))
                                            .items_center()
                                            .child(
                                                h_flex()
                                                    .id("checkbox-modality-text")
                                                    .gap(px(6.))
                                                    .items_center()
                                                    .cursor_pointer()
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.toggle_zcode_modality_text(cx);
                                                    }))
                                                    .child(
                                                        div()
                                                            .size(px(18.))
                                                            .rounded(px(4.))
                                                            .border_1()
                                                            .border_color(if text_checked { theme.primary } else { theme.border })
                                                            .bg(if text_checked { theme.primary } else { gpui::transparent_black() })
                                                            .flex()
                                                            .items_center()
                                                            .justify_center()
                                                            .when(text_checked, |this| {
                                                                this.child(
                                                                    Icon::new(IconName::Check)
                                                                        .size(px(13.))
                                                                        .text_color(rgb(0xFFFFFF)),
                                                                )
                                                            }),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_size(px(13.))
                                                            .text_color(theme.foreground)
                                                            .child(t!("provider.modality_text").to_string()),
                                                    ),
                                            )
                                            .child(
                                                h_flex()
                                                    .id("checkbox-modality-image")
                                                    .gap(px(6.))
                                                    .items_center()
                                                    .cursor_pointer()
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.toggle_zcode_modality_image(cx);
                                                    }))
                                                    .child(
                                                        div()
                                                            .size(px(18.))
                                                            .rounded(px(4.))
                                                            .border_1()
                                                            .border_color(if image_checked { theme.primary } else { theme.border })
                                                            .bg(if image_checked { theme.primary } else { gpui::transparent_black() })
                                                            .flex()
                                                            .items_center()
                                                            .justify_center()
                                                            .when(image_checked, |this| {
                                                                this.child(
                                                                    Icon::new(IconName::Check)
                                                                        .size(px(13.))
                                                                        .text_color(rgb(0xFFFFFF)),
                                                                )
                                                            }),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_size(px(13.))
                                                            .text_color(theme.foreground)
                                                            .child(t!("provider.modality_image").to_string()),
                                                    ),
                                            ),
                                    ),
                            )
                        })
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(theme.muted_foreground)
                                .child(t!("provider.clipboard_helper_tip").to_string()),
                        ),
                ),
            )
            .when(form.app != AppKind::WorkBuddy, |this| {
                this.child(
                // Model Mapping Card
                theme::tile(cx).child(
                    v_flex()
                        .w_full()
                        .gap(px(12.))
                        .child(
                            h_flex()
                                .w_full()
                                .items_center()
                                .justify_between()
                                .child(
                                    v_flex()
                                        .gap(px(2.))
                                        .child(theme::tile_label("MODEL MAPPING / 模型映射 (可选)", cx))
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .text_color(theme.muted_foreground)
                                                .child("自定义在客户端下拉菜单中展示的模型别名、映射到服务商的真实模型、上下文窗口大小及思考等级。"),
                                        ),
                                )
                                .child(
                                    if form.has_fetched_models || !form.catalog_rows.is_empty() {
                                        Button::new("add-mapping-btn")
                                            .primary()
                                            .small()
                                            .icon(IconName::Plus)
                                            .label(t!("provider.add_mapping").to_string())
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.add_catalog_row(window, cx);
                                            }))
                                            .into_any_element()
                                    } else {
                                        Button::new("fetch-catalog-btn")
                                            .outline()
                                            .small()
                                            .icon(IconName::ArrowDown)
                                            .label("拉取模型")
                                            .disabled(form.is_fetching_models)
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.fetch_models_for_form(window, cx);
                                            }))
                                            .into_any_element()
                                    },
                                ),
                        )
                        .child(
                            if form.catalog_rows.is_empty() {
                                v_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_center()
                                    .py(px(20.))
                                    .gap(px(6.))
                                    .rounded(px(8.))
                                    .border_1()
                                    .border_color(theme.border)
                                    .bg(theme.secondary.opacity(0.3))
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .text_color(theme.muted_foreground)
                                            .child("暂无模型映射配置"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(theme.muted_foreground)
                                            .child(if form.has_fetched_models {
                                                "已成功获取可用模型列表，点击右上角「新增映射」添加映射规则"
                                            } else {
                                                "点击「拉取模型」获取服务商可用模型，或点击上方「新增映射」直接添加"
                                            }),
                                    )
                                    .into_any_element()
                            } else {
                                v_flex()
                                    .w_full()
                                    .gap(px(6.))
                                    .child(
                                        h_flex()
                                            .w_full()
                                            .items_center()
                                            .gap(px(10.))
                                            .px(px(8.))
                                            .py(px(6.))
                                            .rounded(px(6.))
                                            .bg(theme.secondary.opacity(0.6))
                                            .child(
                                                div()
                                                    .w(px(200.))
                                                    .text_size(px(12.))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(theme.muted_foreground)
                                                    .child("菜单显示名"),
                                            )
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .text_size(px(12.))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(theme.muted_foreground)
                                                    .child("实际请求模型"),
                                            )
                                            .child(
                                                div()
                                                    .w(px(120.))
                                                    .text_size(px(12.))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(theme.muted_foreground)
                                                    .child("上下文窗口"),
                                            )
                                            .child(
                                                div()
                                                    .w(px(130.))
                                                    .text_size(px(12.))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(theme.muted_foreground)
                                                    .child("思考等级"),
                                            )
                                            .child(
                                                div()
                                                    .w(px(36.))
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .text_size(px(12.))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(theme.muted_foreground)
                                                    .child("操作"),
                                            ),
                                    )
                                    .children(
                                        form.catalog_rows.iter().enumerate().map(|(idx, row)| {
                                            h_flex()
                                                .w_full()
                                                .items_center()
                                                .gap(px(10.))
                                                .px(px(4.))
                                                .py(px(3.))
                                                .child(
                                                    div()
                                                        .w(px(200.))
                                                        .child(Input::new(&row.display_name).small().cleanable(true)),
                                                )
                                                .child(
                                                    h_flex()
                                                        .flex_1()
                                                        .items_center()
                                                        .gap(px(6.))
                                                        .child(
                                                            div().flex_1().child(Input::new(&row.model).small().cleanable(true))
                                                        )
                                                        .when_some(row.model_select.as_ref(), |this, select| {
                                                            this.child(
                                                                div()
                                                                    .w(px(160.))
                                                                    .child(
                                                                        Select::new(select)
                                                                            .small()
                                                                            .placeholder("选择模型")
                                                                            .search_placeholder("搜索模型..."),
                                                                    ),
                                                            )
                                                        }),
                                                )
                                                .child(
                                                    div()
                                                        .w(px(120.))
                                                        .child(Input::new(&row.context_window).small()),
                                                )
                                                .child(
                                                    div()
                                                        .w(px(130.))
                                                        .child(Select::new(&row.reasoning_effort).small()),
                                                )
                                                .child(
                                                    div()
                                                        .w(px(36.))
                                                        .flex()
                                                        .items_center()
                                                        .justify_center()
                                                        .child(
                                                            Button::new(SharedString::from(format!("del-row-{}", idx)))
                                                                .ghost()
                                                                .small()
                                                                .icon(IconName::Delete)
                                                                .tooltip("删除此模型映射")
                                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                                    this.remove_catalog_row(idx, cx);
                                                                })),
                                                        ),
                                                )
                                        }),
                                    )
                                    .into_any_element()
                            },
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(theme.muted_foreground)
                                .child("💡 提示：配置模型映射后，在客户端下拉菜单或设置中可直接切换已配置的模型。"),
                        ),
                ),
            )})
            .child(
                // Bottom Action Buttons
                h_flex()
                    .w_full()
                    .justify_end()
                    .gap(px(10.))
                    .pt(px(8.))
                    .pb(px(24.))
                    .child(
                        Button::new("bottom-cancel-btn")
                            .outline()
                            .label(t!("provider.cancel").to_string())
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.form = None;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("bottom-save-btn")
                            .primary()
                            .icon(IconName::Check)
                            .label(t!("provider.save").to_string())
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.submit_form(window, cx);
                            })),
                    ),
            )
            .into_any_element()
    }
}

impl Render for RouterApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dark = cx.theme().is_dark();
        let page = if self.form.is_some() {
            self.render_form_page(cx).into_any_element()
        } else {
            match self.route {
                Route::Dashboard => self.render_dashboard_page(cx).into_any_element(),
                Route::Codex => self
                    .render_app_providers_page(AppKind::Codex, cx)
                    .into_any_element(),
                Route::Claude => self
                    .render_app_providers_page(AppKind::Claude, cx)
                    .into_any_element(),
                Route::Grok => self
                    .render_app_providers_page(AppKind::Grok, cx)
                    .into_any_element(),
                Route::OpenCode => self
                    .render_app_providers_page(AppKind::OpenCode, cx)
                    .into_any_element(),
                Route::Pi => self
                    .render_app_providers_page(AppKind::Pi, cx)
                    .into_any_element(),
                Route::Cursor => self
                    .render_app_providers_page(AppKind::Cursor, cx)
                    .into_any_element(),
                Route::ZCode => self
                    .render_app_providers_page(AppKind::ZCode, cx)
                    .into_any_element(),
                Route::WorkBuddy => self
                    .render_app_providers_page(AppKind::WorkBuddy, cx)
                    .into_any_element(),
                Route::Notifications => self.render_notifications_page(cx).into_any_element(),
                Route::Settings => self.render_settings_page(cx).into_any_element(),
            }
        };

        div()
            .size_full()
            .relative()
            .child(
                h_flex()
                    .size_full()
                    .relative()
                    .bg(cx.theme().sidebar)
                    .text_color(cx.theme().foreground)
                    .font_family(".SystemUIFont")
                    .key_context("RouterApp")
                    .on_key_down(
                        cx.listener(|this, event: &gpui::KeyDownEvent, _window, cx| {
                            if (event.keystroke.modifiers.platform
                                || event.keystroke.modifiers.control)
                                && event.keystroke.key == "b"
                            {
                                this.sidebar_open = !this.sidebar_open;
                                cx.notify();
                            }
                        }),
                    )
                    .when(self.sidebar_open, |this| {
                        if self.route == Route::Settings {
                            this.child(self.render_settings_sidebar(cx))
                        } else {
                            this.child(self.render_sidebar(cx))
                        }
                    })
                    .child(
                        div()
                            .flex_1()
                            .h_full()
                            .min_w_0()
                            .p(px(8.))
                            .when(self.sidebar_open, |this| this.pl(px(0.)))
                            .child(
                                div()
                                    .size_full()
                                    .rounded(px(14.))
                                    .overflow_hidden()
                                    .flex()
                                    .flex_col()
                                    .bg(theme::inset_bg(dark))
                                    .shadow_sm()
                                    .when(!self.sidebar_open, |this| this.pt(px(28.)))
                                    .child(
                                        div()
                                            .id("main-scroll")
                                            .flex_1()
                                            .min_h_0()
                                            .p(px(20.))
                                            .overflow_y_scrollbar()
                                            .child(page),
                                    ),
                            ),
                    )
                    .child(self.render_chrome(window, cx)),
            )
            .children(gpui_component::Root::render_dialog_layer(window, cx))
            .children(gpui_component::Root::render_sheet_layer(window, cx))
            .children(gpui_component::Root::render_notification_layer(window, cx))
    }
}

impl FormDraft {
    fn create(app: AppKind, window: &mut Window, cx: &mut Context<RouterApp>) -> Self {
        let default_form = match app {
            AppKind::Codex => ProviderForm::Codex(CodexForm {
                name: String::new(),
                website_url: String::new(),
                kind: CodexKind::ResponsesThirdParty,
                api_key: String::new(),
                base_url: String::new(),
                model: DEFAULT_CODEX_MODEL.to_string(),
                model_mappings: Vec::new(),
            }),
            AppKind::Claude => ProviderForm::Claude(ClaudeForm {
                name: String::new(),
                website_url: String::new(),
                kind: ClaudeKind::ThirdParty,
                api_key: String::new(),
                base_url: String::new(),
                model: DEFAULT_CLAUDE_MODEL.to_string(),
                model_mappings: Vec::new(),
            }),
            AppKind::Grok => ProviderForm::Grok(GrokForm {
                name: String::new(),
                website_url: String::new(),
                kind: GrokKind::ThirdParty,
                api_key: String::new(),
                base_url: String::new(),
                model: DEFAULT_GROK_MODEL.to_string(),
                model_mappings: Vec::new(),
            }),
            AppKind::OpenCode => ProviderForm::OpenCode(OpenCodeForm {
                name: String::new(),
                website_url: String::new(),
                kind: OpenCodeKind::ThirdParty,
                npm: domain::DEFAULT_OPENCODE_NPM.to_string(),
                api_key: String::new(),
                base_url: String::new(),
                model: DEFAULT_OPENCODE_MODEL.to_string(),
                model_mappings: Vec::new(),
            }),
            AppKind::Pi => ProviderForm::Pi(PiForm {
                name: String::new(),
                website_url: String::new(),
                kind: PiKind::ThirdParty,
                api_type: domain::DEFAULT_PI_API_TYPE.to_string(),
                api_key: String::new(),
                base_url: String::new(),
                model: DEFAULT_PI_MODEL.to_string(),
                model_mappings: Vec::new(),
            }),
            AppKind::Cursor => ProviderForm::Cursor(CursorForm {
                name: String::new(),
                website_url: String::new(),
                kind: CursorKind::ThirdParty,
                provider_type: domain::DEFAULT_CURSOR_PROVIDER_TYPE.to_string(),
                api_key: String::new(),
                base_url: String::new(),
                model: DEFAULT_CURSOR_MODEL.to_string(),
                model_mappings: Vec::new(),
            }),
            AppKind::ZCode => ProviderForm::ZCode(ZCodeForm {
                name: String::new(),
                website_url: String::new(),
                kind: ZCodeKind::ThirdParty,
                provider_kind: DEFAULT_ZCODE_PROVIDER_KIND.to_string(),
                api_key: String::new(),
                base_url: String::new(),
                model: DEFAULT_ZCODE_MODEL.to_string(),
                modality_text: true,
                modality_image: true,
                model_mappings: Vec::new(),
            }),
            AppKind::WorkBuddy => ProviderForm::WorkBuddy(WorkBuddyForm {
                name: String::new(),
                website_url: String::new(),
                kind: WorkBuddyKind::ThirdParty,
                model_id: DEFAULT_WORKBUDDY_MODEL.to_string(),
                vendor: DEFAULT_WORKBUDDY_VENDOR.to_string(),
                base_url: String::new(),
                api_key: String::new(),
                supports_tool_call: true,
                supports_images: true,
                supports_reasoning: false,
                reasoning_only: false,
                can_disable_reasoning: true,
                use_custom_protocol: false,
                max_input_tokens: Some(262144),
                max_output_tokens: Some(65536),
                reasoning_effort: "medium".to_string(),
                supported_reasoning_efforts: vec!["medium".to_string()],
            }),
        };

        Self::from_provider_form(app, None, default_form, window, cx)
    }

    fn from_provider_form(
        app: AppKind,
        editing_id: Option<String>,
        form: ProviderForm,
        window: &mut Window,
        cx: &mut Context<RouterApp>,
    ) -> Self {
        let presets = presets_for_app(app);

        let (name, api_key, base_url, model, is_official, catalog_rows_data): (
            String,
            String,
            String,
            String,
            bool,
            Vec<(String, String, Option<u64>, Option<String>)>,
        ) = match &form {
            ProviderForm::Codex(f) => (
                f.name.clone(),
                f.api_key.clone(),
                f.base_url.clone(),
                f.model.clone(),
                f.kind.is_official(),
                f.model_mappings
                    .iter()
                    .map(|m| {
                        (
                            m.display_name.clone(),
                            m.model.clone(),
                            m.context_window,
                            m.reasoning_effort.clone(),
                        )
                    })
                    .collect(),
            ),
            ProviderForm::Claude(f) => (
                f.name.clone(),
                f.api_key.clone(),
                f.base_url.clone(),
                f.model.clone(),
                f.kind.is_official(),
                f.model_mappings
                    .iter()
                    .map(|m| {
                        (
                            m.display_name.clone(),
                            m.model.clone(),
                            m.context_window,
                            m.reasoning_effort.clone(),
                        )
                    })
                    .collect(),
            ),
            ProviderForm::Grok(f) => (
                f.name.clone(),
                f.api_key.clone(),
                f.base_url.clone(),
                f.model.clone(),
                f.kind.is_official(),
                f.model_mappings
                    .iter()
                    .map(|m| {
                        (
                            m.display_name.clone(),
                            m.model.clone(),
                            m.context_window,
                            m.reasoning_effort.clone(),
                        )
                    })
                    .collect(),
            ),
            ProviderForm::OpenCode(f) => (
                f.name.clone(),
                f.api_key.clone(),
                f.base_url.clone(),
                f.model.clone(),
                f.kind.is_official(),
                f.model_mappings
                    .iter()
                    .map(|m| {
                        (
                            m.display_name.clone(),
                            m.model_id.clone(),
                            m.context_limit,
                            None,
                        )
                    })
                    .collect(),
            ),
            ProviderForm::Pi(f) => (
                f.name.clone(),
                f.api_key.clone(),
                f.base_url.clone(),
                f.model.clone(),
                f.kind.is_official(),
                f.model_mappings
                    .iter()
                    .map(|m| {
                        (
                            m.display_name.clone(),
                            m.model_id.clone(),
                            m.context_window,
                            None,
                        )
                    })
                    .collect(),
            ),
            ProviderForm::Cursor(f) => (
                f.name.clone(),
                f.api_key.clone(),
                f.base_url.clone(),
                f.model.clone(),
                f.kind.is_official(),
                f.model_mappings
                    .iter()
                    .map(|m| {
                        (
                            m.display_name.clone(),
                            m.model.clone(),
                            m.context_window,
                            m.reasoning_effort.clone(),
                        )
                    })
                    .collect(),
            ),
            ProviderForm::ZCode(f) => (
                f.name.clone(),
                f.api_key.clone(),
                f.base_url.clone(),
                f.model.clone(),
                f.kind.is_official(),
                f.model_mappings
                    .iter()
                    .map(|m| {
                        (
                            m.display_name.clone(),
                            m.model_id.clone(),
                            m.context_limit,
                            None,
                        )
                    })
                    .collect(),
            ),
            ProviderForm::WorkBuddy(f) => (
                f.name.clone(),
                f.api_key.clone(),
                f.base_url.clone(),
                f.model_id.clone(),
                f.kind.is_official(),
                Vec::new(),
            ),
        };

        let selected_index = if editing_id.is_none() {
            presets
                .iter()
                .position(|p| p.id == "custom")
                .map(|idx| gpui_component::IndexPath::default().row(idx))
        } else if name.trim().is_empty() {
            None
        } else {
            presets
                .iter()
                .position(|p| p.name == name)
                .map(|idx| gpui_component::IndexPath::default().row(idx))
        };

        let presets_for_sub = presets.clone();
        let preset_select =
            cx.new(|cx| SelectState::new(presets, selected_index, window, cx).searchable(true));

        let view = cx.entity();
        let _preset_sub = window.subscribe(
            &preset_select,
            cx,
            move |_, event: &SelectEvent<Vec<PresetSelectItem>>, window, cx| {
                if let SelectEvent::Confirm(Some(preset_id)) = event {
                    if let Some(preset) = presets_for_sub.iter().find(|p| p.id == *preset_id) {
                        let p = preset.clone();
                        view.update(cx, |this, cx| {
                            this.apply_preset(p, window, cx);
                        });
                    }
                }
            },
        );

        let catalog_rows = catalog_rows_data
            .into_iter()
            .map(|(dn, m, cw, re)| {
                CatalogRowDraft::new(&dn, &m, cw, re.as_deref(), &[], window, cx)
            })
            .collect();

        let (zcode_modality_text, zcode_modality_image) = match &form {
            ProviderForm::ZCode(f) => (f.modality_text, f.modality_image),
            _ => (true, true),
        };

        let (
            workbuddy_supports_tool_call,
            workbuddy_supports_images,
            workbuddy_supports_reasoning,
            workbuddy_reasoning_only,
            workbuddy_can_disable_reasoning,
            workbuddy_use_custom_protocol,
            workbuddy_max_input_tokens_str,
            workbuddy_max_output_tokens_str,
            workbuddy_reasoning_effort,
            workbuddy_supported_effort_low,
            workbuddy_supported_effort_medium,
            workbuddy_supported_effort_high,
            workbuddy_supported_effort_xhigh,
            workbuddy_supported_effort_max,
        ) = match &form {
            ProviderForm::WorkBuddy(f) => {
                let effort = if f.reasoning_effort.is_empty() {
                    "medium".to_string()
                } else {
                    f.reasoning_effort.clone()
                };
                let has_efforts = !f.supported_reasoning_efforts.is_empty();
                (
                    f.supports_tool_call,
                    f.supports_images,
                    f.supports_reasoning,
                    f.reasoning_only,
                    f.can_disable_reasoning,
                    f.use_custom_protocol,
                    f.max_input_tokens
                        .map(|v| v.to_string())
                        .unwrap_or_default(),
                    f.max_output_tokens
                        .map(|v| v.to_string())
                        .unwrap_or_default(),
                    effort.clone(),
                    if has_efforts {
                        f.supported_reasoning_efforts.iter().any(|s| s == "low")
                    } else {
                        effort == "low"
                    },
                    if has_efforts {
                        f.supported_reasoning_efforts.iter().any(|s| s == "medium")
                    } else {
                        true
                    },
                    if has_efforts {
                        f.supported_reasoning_efforts.iter().any(|s| s == "high")
                    } else {
                        effort == "high"
                    },
                    if has_efforts {
                        f.supported_reasoning_efforts.iter().any(|s| s == "xhigh")
                    } else {
                        effort == "xhigh"
                    },
                    if has_efforts {
                        f.supported_reasoning_efforts.iter().any(|s| s == "max")
                    } else {
                        effort == "max"
                    },
                )
            }
            _ => (
                true,
                true,
                false,
                false,
                true,
                false,
                "262144".to_string(),
                "65536".to_string(),
                "medium".to_string(),
                false,
                true,
                false,
                false,
                false,
            ),
        };

        let (workbuddy_reasoning_effort_select, _workbuddy_reasoning_effort_sub) = if app
            == AppKind::WorkBuddy
        {
            let options = vec![
                WorkBuddyReasoningEffortItem {
                    label: "Low (低)".into(),
                    value: "low".into(),
                },
                WorkBuddyReasoningEffortItem {
                    label: "Medium (中)".into(),
                    value: "medium".into(),
                },
                WorkBuddyReasoningEffortItem {
                    label: "High (高)".into(),
                    value: "high".into(),
                },
                WorkBuddyReasoningEffortItem {
                    label: "Extra High (极高)".into(),
                    value: "xhigh".into(),
                },
                WorkBuddyReasoningEffortItem {
                    label: "Max (最大)".into(),
                    value: "max".into(),
                },
            ];
            let selected_idx = options
                .iter()
                .position(|o| o.value == workbuddy_reasoning_effort)
                .or(Some(1))
                .map(|i| gpui_component::IndexPath::default().row(i));
            let select = cx.new(|cx| SelectState::new(options, selected_idx, window, cx));
            let view = cx.entity();
            let sub = window.subscribe(
                &select,
                cx,
                move |_, event: &SelectEvent<Vec<WorkBuddyReasoningEffortItem>>, _window, cx| {
                    if let SelectEvent::Confirm(Some(effort_val)) = event {
                        let effort_str = effort_val.clone();
                        view.update(cx, |this, cx| {
                            if let Some(form) = this.form.as_mut() {
                                form.workbuddy_reasoning_effort = effort_str;
                                cx.notify();
                            }
                        });
                    }
                },
            );
            (Some(select), Some(sub))
        } else {
            (None, None)
        };

        Self {
            app,
            editing_id,
            is_official,
            name: field(window, cx, &name, "输入服务商名称，如 PackyCode"),
            api_key: cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("sk-...")
                    .masked(true)
                    .default_value(api_key)
            }),
            base_url: field(window, cx, &base_url, "https://api.example.com/v1"),
            model: field(window, cx, &model, "例如: gpt-5.6-sol / claude-3-7-sonnet"),
            zcode_modality_text,
            zcode_modality_image,
            workbuddy_supports_tool_call,
            workbuddy_supports_images,
            workbuddy_supports_reasoning,
            workbuddy_reasoning_only,
            workbuddy_can_disable_reasoning,
            workbuddy_use_custom_protocol,
            workbuddy_max_input_tokens: field(
                window,
                cx,
                &workbuddy_max_input_tokens_str,
                "262144",
            ),
            workbuddy_max_output_tokens: field(
                window,
                cx,
                &workbuddy_max_output_tokens_str,
                "65536",
            ),
            workbuddy_reasoning_effort,
            workbuddy_reasoning_effort_select,
            workbuddy_supported_effort_low,
            workbuddy_supported_effort_medium,
            workbuddy_supported_effort_high,
            workbuddy_supported_effort_xhigh,
            workbuddy_supported_effort_max,
            preset_select,
            catalog_rows,
            fetched_models: Vec::new(),
            has_fetched_models: false,
            default_model_select: None,
            is_fetching_models: false,
            is_testing_connectivity: false,
            connectivity_result: None,
            _preset_sub: Some(_preset_sub),
            _default_model_sub: None,
            _workbuddy_reasoning_effort_sub,
        }
    }

    pub fn apply_clipboard_info(
        &mut self,
        info: ClipboardProviderInfo,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(name) = info.name {
            let cur_name = self.name.read(cx).value().to_string();
            if cur_name.trim().is_empty() || cur_name == "PackyCode" || cur_name == "Custom" {
                self.name
                    .update(cx, |input, cx| input.set_value(name, window, cx));
            }
        }
        self.api_key
            .update(cx, |input, cx| input.set_value(info.api_key, window, cx));
        self.base_url
            .update(cx, |input, cx| input.set_value(info.base_url, window, cx));
        if let Some(model) = info.model {
            self.model
                .update(cx, |input, cx| input.set_value(model, window, cx));
        }
        if !info.models.is_empty() {
            self.fetched_models = info.models.clone();
            self.has_fetched_models = true;
            let items: Vec<ModelSelectItem> = info
                .models
                .iter()
                .map(|m| ModelSelectItem { name: m.clone() })
                .collect();
            let current_model = self.model.read(cx).value().to_string();
            let selected_idx = info
                .models
                .iter()
                .position(|m| m == &current_model)
                .map(|i| gpui_component::IndexPath::default().row(i));
            let default_select =
                cx.new(|cx| SelectState::new(items, selected_idx, window, cx).searchable(true));
            let form_model_state = self.model.clone();
            let default_sub = window.subscribe(
                &default_select,
                cx,
                move |_, event: &SelectEvent<Vec<ModelSelectItem>>, window, cx| {
                    if let SelectEvent::Confirm(Some(m)) = event {
                        let val = m.clone();
                        form_model_state.update(cx, |input, cx| {
                            input.set_value(val, window, cx);
                        });
                    }
                },
            );
            self.default_model_select = Some(default_select);
            self._default_model_sub = Some(default_sub);
            for row in &mut self.catalog_rows {
                row.set_fetched_models(&info.models, window, cx);
            }
        }
    }

    fn to_provider_form(&self, cx: &App) -> ProviderForm {
        let name = self.name.read(cx).value().to_string();
        let api_key = self.api_key.read(cx).value().to_string();
        let base_url = self.base_url.read(cx).value().to_string();
        let model = self.model.read(cx).value().to_string();

        match self.app {
            AppKind::Codex => {
                let model_mappings = self
                    .catalog_rows
                    .iter()
                    .filter_map(|row| row.to_codex_mapping(cx))
                    .collect();
                ProviderForm::Codex(CodexForm {
                    name,
                    website_url: String::new(),
                    kind: if self.is_official {
                        CodexKind::Official
                    } else {
                        CodexKind::ResponsesThirdParty
                    },
                    api_key,
                    base_url,
                    model,
                    model_mappings,
                })
            }
            AppKind::Claude => {
                let model_mappings = self
                    .catalog_rows
                    .iter()
                    .filter_map(|row| row.to_claude_mapping(cx))
                    .collect();
                ProviderForm::Claude(ClaudeForm {
                    name,
                    website_url: String::new(),
                    kind: if self.is_official {
                        ClaudeKind::Official
                    } else {
                        ClaudeKind::ThirdParty
                    },
                    api_key,
                    base_url,
                    model,
                    model_mappings,
                })
            }
            AppKind::Grok => {
                let model_mappings = self
                    .catalog_rows
                    .iter()
                    .filter_map(|row| row.to_grok_mapping(cx))
                    .collect();
                ProviderForm::Grok(GrokForm {
                    name,
                    website_url: String::new(),
                    kind: if self.is_official {
                        GrokKind::Official
                    } else {
                        GrokKind::ThirdParty
                    },
                    api_key,
                    base_url,
                    model,
                    model_mappings,
                })
            }
            AppKind::OpenCode => {
                let model_mappings = self
                    .catalog_rows
                    .iter()
                    .filter_map(|row| row.to_opencode_mapping(cx))
                    .collect();
                ProviderForm::OpenCode(OpenCodeForm {
                    name,
                    website_url: String::new(),
                    kind: if self.is_official {
                        OpenCodeKind::Official
                    } else {
                        OpenCodeKind::ThirdParty
                    },
                    npm: domain::DEFAULT_OPENCODE_NPM.to_string(),
                    api_key,
                    base_url,
                    model,
                    model_mappings,
                })
            }
            AppKind::Pi => {
                let model_mappings = self
                    .catalog_rows
                    .iter()
                    .filter_map(|row| row.to_pi_mapping(cx))
                    .collect();
                ProviderForm::Pi(PiForm {
                    name,
                    website_url: String::new(),
                    kind: if self.is_official {
                        PiKind::Official
                    } else {
                        PiKind::ThirdParty
                    },
                    api_type: domain::DEFAULT_PI_API_TYPE.to_string(),
                    api_key,
                    base_url,
                    model,
                    model_mappings,
                })
            }
            AppKind::Cursor => {
                let model_mappings = self
                    .catalog_rows
                    .iter()
                    .filter_map(|row| row.to_cursor_mapping(cx))
                    .collect();
                ProviderForm::Cursor(CursorForm {
                    name,
                    website_url: String::new(),
                    kind: if self.is_official {
                        CursorKind::Official
                    } else {
                        CursorKind::ThirdParty
                    },
                    provider_type: domain::DEFAULT_CURSOR_PROVIDER_TYPE.to_string(),
                    api_key,
                    base_url,
                    model,
                    model_mappings,
                })
            }
            AppKind::ZCode => {
                let model_mappings = self
                    .catalog_rows
                    .iter()
                    .filter_map(|row| row.to_zcode_mapping(cx))
                    .collect();
                ProviderForm::ZCode(ZCodeForm {
                    name,
                    website_url: String::new(),
                    kind: if self.is_official {
                        ZCodeKind::Official
                    } else {
                        ZCodeKind::ThirdParty
                    },
                    provider_kind: DEFAULT_ZCODE_PROVIDER_KIND.to_string(),
                    api_key,
                    base_url,
                    model,
                    modality_text: self.zcode_modality_text,
                    modality_image: self.zcode_modality_image,
                    model_mappings,
                })
            }
            AppKind::WorkBuddy => {
                let max_in = self
                    .workbuddy_max_input_tokens
                    .read(cx)
                    .value()
                    .trim()
                    .parse::<u64>()
                    .ok();
                let max_out = self
                    .workbuddy_max_output_tokens
                    .read(cx)
                    .value()
                    .trim()
                    .parse::<u64>()
                    .ok();
                let mut supported_efforts = Vec::new();
                if self.workbuddy_supported_effort_low {
                    supported_efforts.push("low".to_string());
                }
                if self.workbuddy_supported_effort_medium {
                    supported_efforts.push("medium".to_string());
                }
                if self.workbuddy_supported_effort_high {
                    supported_efforts.push("high".to_string());
                }
                if self.workbuddy_supported_effort_xhigh {
                    supported_efforts.push("xhigh".to_string());
                }
                if self.workbuddy_supported_effort_max {
                    supported_efforts.push("max".to_string());
                }
                if supported_efforts.is_empty() {
                    supported_efforts.push("medium".to_string());
                }

                ProviderForm::WorkBuddy(WorkBuddyForm {
                    name,
                    website_url: String::new(),
                    kind: if self.is_official {
                        WorkBuddyKind::Official
                    } else {
                        WorkBuddyKind::ThirdParty
                    },
                    model_id: model,
                    vendor: DEFAULT_WORKBUDDY_VENDOR.to_string(),
                    base_url,
                    api_key,
                    supports_tool_call: self.workbuddy_supports_tool_call,
                    supports_images: self.workbuddy_supports_images,
                    supports_reasoning: self.workbuddy_supports_reasoning,
                    reasoning_only: self.workbuddy_reasoning_only,
                    can_disable_reasoning: self.workbuddy_can_disable_reasoning,
                    use_custom_protocol: self.workbuddy_use_custom_protocol,
                    max_input_tokens: max_in,
                    max_output_tokens: max_out,
                    reasoning_effort: if self.workbuddy_reasoning_effort.is_empty() {
                        "medium".to_string()
                    } else {
                        self.workbuddy_reasoning_effort.clone()
                    },
                    supported_reasoning_efforts: supported_efforts,
                })
            }
        }
    }
}

fn field(
    window: &mut Window,
    cx: &mut Context<RouterApp>,
    value: &str,
    placeholder: &str,
) -> Entity<InputState> {
    let value = value.to_string();
    let placeholder = placeholder.to_string();
    cx.new(|cx| {
        InputState::new(window, cx)
            .placeholder(placeholder)
            .default_value(value)
    })
}

fn notify_success(message: impl Into<SharedString>, window: &mut Window, cx: &mut App) {
    window.push_notification(Notification::success(message), cx);
}

fn notify_info(message: impl Into<SharedString>, window: &mut Window, cx: &mut App) {
    window.push_notification(Notification::info(message), cx);
}

fn empty_state(app_name: &str, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    v_flex()
        .w_full()
        .items_center()
        .justify_center()
        .gap(px(12.))
        .py(px(48.))
        .rounded(px(16.))
        .border_1()
        .border_color(theme.border)
        .bg(theme.secondary.opacity(0.4))
        .child(
            div()
                .size(px(48.))
                .rounded(px(12.))
                .bg(theme.border)
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme.muted_foreground)
                .child(Icon::new(IconName::Bot).size(px(24.))),
        )
        .child(
            div()
                .text_size(px(16.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.foreground)
                .child(format!("还没有配置 {} 服务商", app_name)),
        )
        .child(
            div()
                .text_size(px(12.))
                .text_color(theme.muted_foreground)
                .child("你可以点击右上角的「新建服务商」，快速添加并管理服务商。"),
        )
}

fn empty_search_state(cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    v_flex()
        .w_full()
        .items_center()
        .justify_center()
        .gap(px(12.))
        .py(px(48.))
        .rounded(px(16.))
        .border_1()
        .border_color(theme.border)
        .bg(theme.secondary.opacity(0.4))
        .child(
            div()
                .size(px(40.))
                .rounded(px(10.))
                .bg(theme.border)
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme.muted_foreground)
                .child(Icon::new(IconName::Search).size(px(20.))),
        )
        .child(
            div()
                .text_size(px(14.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.foreground)
                .child("未找到匹配的服务商"),
        )
        .child(
            div()
                .text_size(px(12.))
                .text_color(theme.muted_foreground)
                .child("请尝试修改搜索词，或清空搜索栏查看所有服务商。"),
        )
}

fn form_field(label: &str, field: impl IntoElement) -> impl IntoElement {
    v_flex()
        .w_full()
        .gap(px(4.))
        .child(
            div()
                .text_size(px(12.))
                .font_weight(FontWeight::MEDIUM)
                .child(label.to_string()),
        )
        .child(field)
}
