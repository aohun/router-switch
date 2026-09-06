use domain::{
    check_app_update, extract_claude_base_url, extract_claude_model, extract_codex_base_url,
    extract_codex_model, extract_cursor_base_url, extract_cursor_model, extract_grok_base_url,
    extract_grok_model, extract_opencode_base_url, extract_opencode_model, extract_zcode_base_url,
    extract_zcode_model, parse_clipboard_provider_info, AppKind, AppRelease, ClaudeForm,
    ClaudeKind, ClaudeModelMapping, ClipboardProviderInfo, CodexForm, CodexKind, CodexModelMapping,
    CursorForm, CursorKind, CursorModelMapping, GrokForm, GrokKind, GrokModelMapping, OpenCodeForm,
    OpenCodeKind, OpenCodeModelMapping, PiForm, PiKind, PiModelMapping, Provider, ProviderForm,
    ProviderSettings, RequestProtocol, ToolEnvironmentStatus, WorkBuddyForm, WorkBuddyKind,
    ZCodeForm, ZCodeKind, ZCodeModelMapping, CLAUDE_PRESETS, CURSOR_PRESETS, DEFAULT_CLAUDE_MODEL,
    DEFAULT_CODEX_MODEL, DEFAULT_CURSOR_MODEL, DEFAULT_GROK_MODEL, DEFAULT_OPENCODE_MODEL,
    DEFAULT_PI_MODEL, DEFAULT_THINKING_EFFORT, DEFAULT_WORKBUDDY_MODEL, DEFAULT_WORKBUDDY_VENDOR,
    DEFAULT_ZCODE_MODEL, DEFAULT_ZCODE_PROVIDER_KIND, GROK_PRESETS, OPENCODE_PRESETS, PI_PRESETS,
    RESPONSES_PRESETS, THINKING_EFFORTS, WORKBUDDY_PRESETS, ZCODE_PRESETS,
};
use gpui::{
    div, hsla, prelude::FluentBuilder, px, rgb, rgba, uniform_list, AnyElement, App, AppContext,
    Context, Entity, FontWeight, Hsla, InteractiveElement, IntoElement, ParentElement,
    PathPromptOptions, Render, SharedString, StatefulInteractiveElement, Styled, Subscription,
    Window, WindowControlArea,
};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    dialog::DialogButtonProps,
    h_flex,
    input::{Input, InputState},
    notification::Notification,
    scroll::ScrollableElement,
    select::{Select, SelectEvent, SelectItem, SelectState},
    tag::Tag,
    tooltip::Tooltip,
    v_flex, ActiveTheme, Disableable as _, Icon, IconName, Selectable as _, Sizable as _,
    WindowExt,
};
use rust_i18n::t;
use session::Workspace;
use std::path::PathBuf;
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
    let mut presets: Vec<PresetSelectItem> = match app {
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
        AppKind::Claude | AppKind::ClaudeDesktop => CLAUDE_PRESETS
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
    };
    // 自定义模板排第一, 便于直接手填
    if let Some(pos) = presets.iter().position(|p| p.id == "custom") {
        let custom = presets.remove(pos);
        presets.insert(0, custom);
    }
    presets
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolSelectItem {
    pub value: String,
    pub label: String,
}

impl SelectItem for ProtocolSelectItem {
    type Value = String;

    fn title(&self) -> SharedString {
        self.label.clone().into()
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

/// Claude Desktop 接入方式下拉项
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DesktopModeItem {
    pub value: String,
    pub label: String,
}

impl SelectItem for DesktopModeItem {
    type Value = String;

    fn title(&self) -> SharedString {
        self.label.clone().into()
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

fn protocol_items() -> Vec<ProtocolSelectItem> {
    RequestProtocol::all()
        .into_iter()
        .map(|protocol| ProtocolSelectItem {
            value: protocol.as_str().to_string(),
            label: protocol.label().to_string(),
        })
        .collect()
}

fn thinking_effort_items() -> Vec<WorkBuddyReasoningEffortItem> {
    THINKING_EFFORTS
        .into_iter()
        .map(|(value, label)| WorkBuddyReasoningEffortItem {
            label: (*label).into(),
            value: value.to_string(),
        })
        .collect()
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
    /// Codex 专属: 该模型声明支持的思考档位; None = 默认三档
    pub reasoning_levels: Option<Vec<String>>,
    pub model_select: Option<Entity<SelectState<Vec<ModelSelectItem>>>>,
    pub _model_select_sub: Option<Subscription>,
}

/// Claude Desktop 模型映射模式的固定角色行(Sonnet/Opus/Fable/Haiku)
struct DesktopRoleDraft {
    display_name: Entity<InputState>,
    model: Entity<InputState>,
    one_m: bool,
    model_select: Option<Entity<SelectState<Vec<ModelSelectItem>>>>,
    _model_select_sub: Option<Subscription>,
}

impl DesktopRoleDraft {
    /// 拉取模型后为该角色挂上模型下拉(保留当前值选中态)
    fn set_fetched_models(&mut self, fetched_models: &[String], window: &mut Window, cx: &mut App) {
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
        let selected_idx = fetched_models
            .iter()
            .position(|m| m == &current_val)
            .map(|i| gpui_component::IndexPath::default().row(i));
        let select =
            cx.new(|cx| SelectState::new(items, selected_idx, window, cx).searchable(true));
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
}

impl CatalogRowDraft {
    pub fn new(
        display_name_val: &str,
        model_val: &str,
        context_window_val: Option<u64>,
        reasoning_effort_val: Option<&str>,
        reasoning_levels_val: Option<Vec<String>>,
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
            reasoning_levels: reasoning_levels_val,
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
            reasoning_levels: self.reasoning_levels.clone(),
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
    Skills,
    Sessions,
    Prompts,
    Codex,
    Claude,
    ClaudeDesktop,
    Grok,
    OpenCode,
    Pi,
    Cursor,
    ZCode,
    WorkBuddy,
    Notifications,
    Settings,
    UsageScript,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsTab {
    #[default]
    General,
    Auth,
    Advanced,
    About,
}

/// Skills 页视图: 已安装列表 / 发现技能
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum SkillsView {
    #[default]
    Installed,
    Discover,
}

/// 发现技能的来源
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum SkillsDiscoverSource {
    #[default]
    Repos,
    SkillsSh,
}

/// 发现列表的安装状态筛选
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum SkillsDiscoverFilter {
    #[default]
    All,
    Installed,
    Uninstalled,
}

/// 未纳管的应用侧 Skill(存在于应用目录但中心库没有)
#[derive(Debug, Clone)]
struct SkillsUnmanaged {
    app: String,
    path: PathBuf,
    dir_name: String,
    name: Option<String>,
    description: Option<String>,
}

/// 进行中的 OAuth 设备码登录
#[derive(Debug, Clone)]
struct OngoingLogin {
    provider: &'static str,
    device_code: String,
    user_code: String,
    token_endpoint: Option<String>,
    /// 轮询截止(unix 秒)
    deadline: i64,
    interval_secs: u64,
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
    preserve_codex_auth: bool,
    unify_codex_history: bool,
    unify_dialog_migrate: bool,
    unify_dialog_restore: bool,
    settings_tab: SettingsTab,
    dashboard_app_filter: Option<AppKind>,
    dashboard_data: Option<DashboardUsageData>,
    is_loading_dashboard: bool,
    usage_breakdown_tab: UsageBreakdownTab,
    usage_window: UsageWindowChoice,
    usage_metric: UsageMetric,
    usage_refresh_interval: UsageRefreshInterval,
    usage_script_provider: Option<String>,
    usage_enabled: bool,
    usage_template: String,
    usage_api_key: Entity<InputState>,
    usage_base_url: Entity<InputState>,
    usage_access_token: Entity<InputState>,
    usage_user_id: Entity<InputState>,
    usage_timeout: Entity<InputState>,
    usage_interval: Entity<InputState>,
    usage_code: Entity<InputState>,
    usage_querying: bool,
    usage_last_result: Option<domain::UsageQueryResult>,
    usage_badges: std::collections::HashMap<String, (domain::UsageQueryResult, i64)>,
    usage_refreshing: std::collections::HashSet<String>,
    codex_oauth_status: Option<session::NativeAuthStatus>,
    xai_oauth_status: Option<session::NativeAuthStatus>,
    oauth_pending: Option<OngoingLogin>,
    sessions_list: Vec<session::sessions::SessionMeta>,
    sessions_loading: bool,
    sessions_filter: Option<String>,
    session_selected: Option<session::sessions::SessionMeta>,
    session_messages: Vec<session::sessions::SessionMessage>,
    session_messages_loading: bool,
    session_checked: std::collections::HashSet<String>,
    sessions_search: Entity<InputState>,
    sessions_batch_mode: bool,
    sessions_search_open: bool,
    sessions_filter_menu_open: bool,
    prompts_app: AppKind,
    prompts_list: Vec<domain::Prompt>,
    prompts_loading: bool,
    prompts_search: Entity<InputState>,
    skills_list: Vec<session::skills::SkillEntry>,
    skills_loading: bool,
    skills_search: Entity<InputState>,
    // Skills 扩展(对齐 cc-switch): 中心库 / 发现 / 备份 / 更新
    skills_view: SkillsView,
    skills_discover_source: SkillsDiscoverSource,
    skills_discover_filter: SkillsDiscoverFilter,
    skills_discover_list: Vec<session::skills::hub::DiscoverableSkill>,
    skills_discover_loading: bool,
    skills_discover_search: Entity<InputState>,
    skills_skills_sh_list: Vec<session::skills::hub::DiscoverableSkill>,
    skills_skills_sh_query: String,
    skills_skills_sh_offset: usize,
    skills_skills_sh_total: usize,
    skills_skills_sh_loading: bool,
    skills_repos: Vec<session::skills::hub::SkillRepo>,
    skills_repo_owner: Entity<InputState>,
    skills_repo_name: Entity<InputState>,
    skills_repo_branch: Entity<InputState>,
    skills_managed_names: Vec<String>,
    skills_unmanaged: Vec<SkillsUnmanaged>,
    skills_import_selected: std::collections::HashSet<String>,
    skills_checking_updates: bool,
    skills_installing_key: Option<String>,
    usage_refresh_select: Entity<SelectState<Vec<UsageRefreshSelectItem>>>,
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
    notes: Entity<InputState>,
    website_url: Entity<InputState>,
    codex_auth_mode: String,
    codex_auth_dropdown_open: bool,
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
    desktop_mode: String,
    desktop_mode_select: Option<Entity<SelectState<Vec<DesktopModeItem>>>>,
    _desktop_mode_sub: Option<Subscription>,
    desktop_roles: Vec<DesktopRoleDraft>,
    fetched_models: Vec<String>,
    has_fetched_models: bool,
    default_model_select: Option<Entity<SelectState<Vec<ModelSelectItem>>>>,
    is_fetching_models: bool,
    is_testing_connectivity: bool,
    connectivity_result: Option<domain::ConnectivityCheckResult>,
    protocol_select: Entity<SelectState<Vec<ProtocolSelectItem>>>,
    _protocol_sub: Option<Subscription>,
    thinking_effort: String,
    thinking_effort_select: Option<Entity<SelectState<Vec<WorkBuddyReasoningEffortItem>>>>,
    _thinking_effort_sub: Option<Subscription>,
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
                Some(gpui_component::IndexPath::default().row(3)),
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
                "claude".into(),
                "codex".into(),
                "cursor".into(),
                "opencode".into(),
                "grok".into(),
                "pi".into(),
                "zcode".into(),
                "workbuddy".into(),
            ];
        }

        let usage_api_key =
            cx.new(|cx| InputState::new(window, cx).placeholder("留空则使用服务商的 API Key"));
        let usage_base_url =
            cx.new(|cx| InputState::new(window, cx).placeholder("留空则使用服务商的请求地址"));
        let usage_access_token = cx.new(|cx| InputState::new(window, cx));
        let usage_user_id = cx.new(|cx| InputState::new(window, cx));
        let usage_timeout = cx.new(|cx| InputState::new(window, cx));
        let usage_interval = cx.new(|cx| InputState::new(window, cx));
        let usage_code = cx.new(|cx| InputState::new(window, cx).code_editor("javascript"));
        let sessions_search =
            cx.new(|cx| InputState::new(window, cx).placeholder(t!("sessions.search").to_string()));
        let prompts_search = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t!("prompts.search_placeholder").to_string())
        });
        let skills_search =
            cx.new(|cx| InputState::new(window, cx).placeholder(t!("skills.search").to_string()));
        let skills_discover_search = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t!("skills.discover_search").to_string())
        });
        let skills_repo_owner = cx.new(|cx| InputState::new(window, cx).placeholder("anthropics"));
        let skills_repo_name = cx.new(|cx| InputState::new(window, cx).placeholder("skills"));
        let skills_repo_branch = cx.new(|cx| InputState::new(window, cx).placeholder("main"));

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
            preserve_codex_auth: settings.preserve_codex_official_auth_on_switch,
            unify_codex_history: settings.unify_codex_session_history,
            unify_dialog_migrate: false,
            unify_dialog_restore: false,
            settings_tab: SettingsTab::General,
            dashboard_app_filter: None,
            dashboard_data: None,
            is_loading_dashboard: false,
            usage_breakdown_tab: UsageBreakdownTab::Model,
            usage_window: UsageWindowChoice::Hours6,
            usage_metric: UsageMetric::Tokens,
            usage_refresh_interval: UsageRefreshInterval::Sec60,
            usage_script_provider: None,
            usage_enabled: false,
            usage_template: domain::TEMPLATE_GENERAL.to_string(),
            usage_api_key,
            usage_base_url,
            usage_access_token,
            usage_user_id,
            usage_timeout,
            usage_interval,
            usage_code,
            usage_querying: false,
            usage_last_result: None,
            usage_badges: std::collections::HashMap::new(),
            usage_refreshing: std::collections::HashSet::new(),
            codex_oauth_status: None,
            xai_oauth_status: None,
            oauth_pending: None,
            sessions_list: Vec::new(),
            sessions_loading: false,
            sessions_filter: None,
            session_selected: None,
            session_messages: Vec::new(),
            session_messages_loading: false,
            session_checked: std::collections::HashSet::new(),
            sessions_search: sessions_search,
            sessions_batch_mode: false,
            sessions_search_open: true,
            sessions_filter_menu_open: false,
            prompts_app: AppKind::Codex,
            prompts_list: Vec::new(),
            prompts_loading: false,
            prompts_search,
            skills_list: Vec::new(),
            skills_loading: false,
            skills_search: skills_search,
            skills_view: SkillsView::Installed,
            skills_discover_source: SkillsDiscoverSource::Repos,
            skills_discover_filter: SkillsDiscoverFilter::All,
            skills_discover_list: Vec::new(),
            skills_discover_loading: false,
            skills_discover_search: skills_discover_search,
            skills_skills_sh_list: Vec::new(),
            skills_skills_sh_query: String::new(),
            skills_skills_sh_offset: 0,
            skills_skills_sh_total: 0,
            skills_skills_sh_loading: false,
            skills_repos: Vec::new(),
            skills_repo_owner: skills_repo_owner,
            skills_repo_name: skills_repo_name,
            skills_repo_branch: skills_repo_branch,
            skills_managed_names: Vec::new(),
            skills_unmanaged: Vec::new(),
            skills_import_selected: std::collections::HashSet::new(),
            skills_checking_updates: false,
            skills_installing_key: None,
            usage_refresh_select,
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
        app.reload_oauth_statuses();
        app.reload_usage_badges();
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

        // 用量查询自动刷新: 每分钟检查一次到期的服务商
        let usage_view = cx.entity().downgrade();
        window
            .spawn(cx, move |cx: &mut gpui::AsyncWindowContext| {
                let mut cx = cx.clone();
                async move {
                    loop {
                        cx.background_executor()
                            .timer(std::time::Duration::from_secs(60))
                            .await;
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs() as i64)
                            .unwrap_or(0);
                        let due: Vec<String> = usage_view
                            .update(&mut cx, |this: &mut RouterApp, _| {
                                this.workspace
                                    .list_usage_scripts()
                                    .unwrap_or_default()
                                    .into_iter()
                                    .filter(|(_, config, fetched_at)| {
                                        if !config.enabled || config.auto_interval_minutes == 0 {
                                            return false;
                                        }
                                        let interval = config.auto_interval_minutes as i64 * 60;
                                        match fetched_at {
                                            Some(at) => now - at >= interval,
                                            None => true,
                                        }
                                    })
                                    .map(|(id, _, _)| id)
                                    .collect()
                            })
                            .unwrap_or_default();
                        for provider_id in due {
                            let prepared = usage_view
                                .update(&mut cx, |this: &mut RouterApp, _| {
                                    this.workspace.prepare_usage_query(&provider_id).ok()
                                })
                                .ok()
                                .flatten();
                            let Some(prepared) = prepared else {
                                continue;
                            };
                            let outcome = session::tokio_runtime()
                                .spawn(prepared.run())
                                .await
                                .map_err(|e| usage_query::UsageQueryError(format!("任务失败: {e}")))
                                .and_then(|inner| inner);
                            let _ = usage_view.update(&mut cx, |this: &mut RouterApp, cx| {
                                this.workspace.complete_usage_query(&provider_id, outcome);
                                this.reload_usage_badges();
                                cx.notify();
                            });
                        }
                    }
                }
            })
            .detach();

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

    fn reload_oauth_statuses(&mut self) {
        self.codex_oauth_status = self.workspace.oauth_status(session::CODEX_PROVIDER).ok();
        self.xai_oauth_status = self.workspace.oauth_status(session::XAI_PROVIDER).ok();
    }

    fn start_oauth_login(
        &mut self,
        provider: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.oauth_pending.is_some() {
            notify_info("已有登录流程进行中, 请先完成或等待超时", window, cx);
            return;
        }
        let start = match self.workspace.oauth_start_login(provider) {
            Ok(start) => start,
            Err(err) => {
                self.fail(err, window, cx);
                return;
            }
        };
        cx.open_url(&start.verification_uri);
        notify_info(
            t!("auth.open_browser", code = start.user_code.as_str()).to_string(),
            window,
            cx,
        );

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        self.oauth_pending = Some(OngoingLogin {
            provider,
            device_code: start.device_code.clone(),
            user_code: start.user_code.clone(),
            token_endpoint: start.token_endpoint.clone(),
            deadline: now + start.expires_in as i64,
            interval_secs: start.interval_secs.max(2),
        });
        cx.notify();

        // 自包含轮询循环: Pending 继续轮, 终态退出
        let view = cx.entity().downgrade();
        window
            .spawn(cx, move |cx: &mut gpui::AsyncWindowContext| {
                let mut cx = cx.clone();
                async move {
                    loop {
                        let Some(login) = view
                            .update(&mut cx, |this, _| this.oauth_pending.clone())
                            .ok()
                            .flatten()
                        else {
                            break;
                        };
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs() as i64)
                            .unwrap_or(0);
                        if now >= login.deadline {
                            let _ = view.update(&mut cx, |this, cx| {
                                this.oauth_pending = None;
                                cx.notify();
                            });
                            let _ = cx.update(|window: &mut Window, cx: &mut App| {
                                window.push_notification(
                                    Notification::warning(t!("auth.login_expired").to_string()),
                                    cx,
                                );
                            });
                            break;
                        }

                        let status = cx
                            .background_executor()
                            .spawn(async move {
                                session::oauth_poll_once(
                                    login.provider,
                                    &login.device_code,
                                    &login.user_code,
                                    login.token_endpoint.as_deref(),
                                )
                            })
                            .await
                            .unwrap_or_else(|e| session::DevicePollStatus::Failed {
                                message: format!("任务失败: {e}"),
                            });

                        match status {
                            session::DevicePollStatus::Pending => {
                                cx.background_executor()
                                    .timer(std::time::Duration::from_secs(
                                        login.interval_secs.max(2),
                                    ))
                                    .await;
                            }
                            session::DevicePollStatus::Complete(tokens) => {
                                let save = view
                                    .update(&mut cx, |this, _| {
                                        this.workspace.oauth_complete(login.provider, &tokens)
                                    })
                                    .map(|inner| inner);
                                let _ = view.update(&mut cx, |this, cx| {
                                    this.oauth_pending = None;
                                    this.reload_oauth_statuses();
                                    cx.notify();
                                });
                                let _ = cx.update(|window: &mut Window, cx: &mut App| match save {
                                    Ok(Ok(())) => window.push_notification(
                                        Notification::success(t!("auth.login_success").to_string()),
                                        cx,
                                    ),
                                    Ok(Err(err)) => window.push_notification(
                                        Notification::error(err.to_string()),
                                        cx,
                                    ),
                                    Err(err) => window.push_notification(
                                        Notification::error(err.to_string()),
                                        cx,
                                    ),
                                });
                                break;
                            }
                            session::DevicePollStatus::Expired
                            | session::DevicePollStatus::AccessDenied => {
                                let _ = view.update(&mut cx, |this, cx| {
                                    this.oauth_pending = None;
                                    cx.notify();
                                });
                                let _ = cx.update(|window: &mut Window, cx: &mut App| {
                                    window.push_notification(
                                        Notification::warning(match status {
                                            session::DevicePollStatus::Expired => {
                                                t!("auth.login_expired").to_string()
                                            }
                                            _ => t!("auth.login_denied").to_string(),
                                        }),
                                        cx,
                                    );
                                });
                                break;
                            }
                            session::DevicePollStatus::Failed { message } => {
                                let _ = view.update(&mut cx, |this, cx| {
                                    this.oauth_pending = None;
                                    cx.notify();
                                });
                                let _ = cx.update(|window: &mut Window, cx: &mut App| {
                                    window.push_notification(
                                        Notification::error(format!("登录失败: {message}")),
                                        cx,
                                    );
                                });
                                break;
                            }
                        }
                    }
                }
            })
            .detach();
    }

    /// 刷新服务商卡片的用量徽标缓存
    pub fn reload_usage_badges(&mut self) {
        self.usage_badges = self
            .workspace
            .list_usage_scripts()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|(id, config, _)| {
                if !config.enabled {
                    return None;
                }
                let (result, fetched_at) = self.workspace.usage_result(&id).ok().flatten()?;
                Some((id, (result, fetched_at)))
            })
            .collect();
    }

    /// 从服务商卡片直接发起一次用量刷新
    fn start_usage_refresh(
        &mut self,
        provider_id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.usage_refreshing.contains(provider_id) {
            return;
        }
        let prepared = match self.workspace.prepare_usage_query(provider_id) {
            Ok(prepared) => prepared,
            Err(_) => return,
        };
        self.usage_refreshing.insert(provider_id.to_string());
        cx.notify();

        let view = cx.entity().downgrade();
        let target = provider_id.to_string();
        window
            .spawn(cx, move |cx: &mut gpui::AsyncWindowContext| {
                let mut cx = cx.clone();
                async move {
                    let outcome = session::tokio_runtime()
                        .spawn(prepared.run())
                        .await
                        .map_err(|e| usage_query::UsageQueryError(format!("任务失败: {e}")))
                        .and_then(|inner| inner);
                    let _ = cx.update(|window: &mut Window, cx: &mut App| {
                        let _ = view.update(cx, |this, cx| {
                            this.usage_refreshing.remove(&target);
                            let result = this.workspace.complete_usage_query(&target, outcome);
                            this.reload_usage_badges();
                            match &result {
                                r if r.success => {
                                    window.push_notification(
                                        Notification::success(usage_summary_text(r)),
                                        cx,
                                    );
                                }
                                r => {
                                    window.push_notification(
                                        Notification::error(
                                            r.error.clone().unwrap_or_else(|| "查询失败".into()),
                                        ),
                                        cx,
                                    );
                                }
                            }
                            cx.notify();
                        });
                    });
                }
            })
            .detach();
    }

    /// 打开用量查询配置页
    fn open_usage_script(
        &mut self,
        provider_id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(_provider) = self.workspace.provider(provider_id).ok().flatten() else {
            return;
        };
        let mut config = self
            .workspace
            .usage_script(provider_id)
            .ok()
            .flatten()
            .unwrap_or_default();
        if config.code.trim().is_empty() {
            config = config.with_preset_code();
        }
        self.usage_script_provider = Some(provider_id.to_string());
        self.usage_enabled = config.enabled;
        self.usage_template = config.template_type.clone();
        self.usage_last_result = self
            .workspace
            .usage_result(provider_id)
            .ok()
            .flatten()
            .map(|(r, _)| r);
        self.usage_querying = false;
        let updates = [
            (
                self.usage_api_key.clone(),
                config.api_key.clone().unwrap_or_default(),
            ),
            (
                self.usage_base_url.clone(),
                config.base_url.clone().unwrap_or_default(),
            ),
            (
                self.usage_access_token.clone(),
                config.access_token.clone().unwrap_or_default(),
            ),
            (
                self.usage_user_id.clone(),
                config.user_id.clone().unwrap_or_default(),
            ),
            (self.usage_timeout.clone(), config.timeout_secs.to_string()),
            (
                self.usage_interval.clone(),
                config.auto_interval_minutes.to_string(),
            ),
            (self.usage_code.clone(), config.code.clone()),
        ];
        for (input, value) in updates {
            input.update(cx, |state, cx| state.set_value(value, window, cx));
        }
        self.set_route(Route::UsageScript, cx);
    }

    fn collect_usage_config(&self, cx: &Context<Self>) -> domain::UsageScriptConfig {
        let read = |input: &Entity<InputState>| input.read(cx).value().to_string();
        let parse_num =
            |raw: String, default: u64| -> u64 { raw.trim().parse().unwrap_or(default) };
        domain::UsageScriptConfig {
            enabled: self.usage_enabled,
            template_type: self.usage_template.clone(),
            code: read(&self.usage_code),
            timeout_secs: parse_num(read(&self.usage_timeout), 10),
            auto_interval_minutes: parse_num(read(&self.usage_interval), 5) as u32,
            api_key: Some(read(&self.usage_api_key)).filter(|s| !s.trim().is_empty()),
            base_url: Some(read(&self.usage_base_url)).filter(|s| !s.trim().is_empty()),
            access_token: Some(read(&self.usage_access_token)).filter(|s| !s.trim().is_empty()),
            user_id: Some(read(&self.usage_user_id)).filter(|s| !s.trim().is_empty()),
        }
    }

    fn set_usage_template(
        &mut self,
        template: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.usage_template = template.clone();
        let code = domain::preset_template(&template).to_string();
        self.usage_code
            .update(cx, |state, cx| state.set_value(code, window, cx));
        cx.notify();
    }

    fn save_usage_script(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(provider_id) = self.usage_script_provider.clone() else {
            return;
        };
        let config = self.collect_usage_config(cx);
        if let Err(err) = self.workspace.save_usage_script(&provider_id, &config) {
            self.fail(err, window, cx);
            return;
        }
        self.reload_usage_badges();
        notify_success(t!("usage_script.saved").to_string(), window, cx);
        let back = if self.previous_route == Route::UsageScript {
            Route::Dashboard
        } else {
            self.previous_route
        };
        self.set_route(back, cx);
    }

    fn test_usage_script(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(provider_id) = self.usage_script_provider.clone() else {
            return;
        };
        let config = self.collect_usage_config(cx);
        let prepared = match self
            .workspace
            .prepare_usage_query_with_config(&provider_id, &config)
        {
            Ok(prepared) => prepared,
            Err(err) => {
                self.fail(
                    session::SessionError::Message(format!("无法发起查询: {err}")),
                    window,
                    cx,
                );
                return;
            }
        };
        self.usage_querying = true;
        cx.notify();

        let view = cx.entity().downgrade();
        window
            .spawn(cx, move |cx: &mut gpui::AsyncWindowContext| {
                let mut cx = cx.clone();
                let provider_id = provider_id.clone();
                async move {
                    let outcome = session::tokio_runtime()
                        .spawn(prepared.run())
                        .await
                        .map_err(|e| usage_query::UsageQueryError(format!("任务失败: {e}")))
                        .and_then(|inner| inner);

                    let _ = cx.update(|window: &mut Window, cx: &mut App| {
                        let _ = view.update(cx, |this, cx| {
                            this.usage_querying = false;
                            let result = this.workspace.complete_usage_query(&provider_id, outcome);
                            this.usage_last_result = Some(result.clone());
                            this.reload_usage_badges();
                            match &result {
                                r if r.success => {
                                    window.push_notification(
                                        Notification::success(usage_summary_text(r)),
                                        cx,
                                    );
                                }
                                r => {
                                    window.push_notification(
                                        Notification::error(
                                            r.error.clone().unwrap_or_else(|| "查询失败".into()),
                                        ),
                                        cx,
                                    );
                                }
                            }
                            cx.notify();
                        });
                    });
                }
            })
            .detach();
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
                    AppKind::ClaudeDesktop => Route::ClaudeDesktop,
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
        for app in AppKind::ALL {
            if let Ok(snapshot) = self.workspace.snapshot_for(*app) {
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
        if route == Route::Sessions && self.sessions_list.is_empty() && !self.sessions_loading {
            self.refresh_sessions(cx);
        }
        if route == Route::Prompts && self.prompts_list.is_empty() && !self.prompts_loading {
            self.refresh_prompts(cx);
        }
        if route == Route::Skills && self.skills_list.is_empty() && !self.skills_loading {
            self.refresh_skills(cx);
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
        self.prompts_search.update(cx, |this, cx| {
            this.set_placeholder(t!("prompts.search_placeholder").to_string(), window, cx);
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

    /// 将 Claude Code 中已有的第三方供应商导入 Claude Desktop
    fn import_claude_desktop_from_claude_code(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self.workspace.import_claude_desktop_from_claude() {
            Ok(0) => {
                notify_info(t!("claude_desktop.import_none").to_string(), window, cx);
            }
            Ok(count) => {
                self.reload();
                notify_success(
                    t!("claude_desktop.import_done", count = count).to_string(),
                    window,
                    cx,
                );
                cx.notify();
            }
            Err(error) => self.fail(error, window, cx),
        }
    }

    fn toggle_preserve_codex_auth(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let new_val = !self.preserve_codex_auth;
        self.preserve_codex_auth = new_val;
        if let Err(err) = self.workspace.set_preserve_codex_official_auth(new_val) {
            self.preserve_codex_auth = !new_val;
            self.fail(err, window, cx);
            return;
        }
        let msg = t!(if new_val {
            "codex_enhance.preserve_enabled"
        } else {
            "codex_enhance.preserve_disabled"
        })
        .to_string();
        notify_success(msg, window, cx);
        cx.notify();
    }

    fn toggle_unify_codex_history(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.unify_codex_history {
            // 开启前确认: 可选把现有官方会话一并迁入(迁移前自动备份)
            self.unify_dialog_migrate = true;
            self.open_unify_enable_dialog(window, cx);
            return;
        }
        // 关闭前探测迁移账本，决定是否提供"按备份恢复"勾选
        let has_backup = self.workspace.has_codex_unify_history_backup();
        self.unify_dialog_restore = has_backup;
        self.open_unify_disable_dialog(window, cx);
    }

    fn open_unify_enable_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let view = cx.entity().downgrade();
        window.open_dialog(cx, move |dialog, _window, cx| {
            let view = view.clone();
            let view_checkbox = view.clone();
            let view_ok = view.clone();

            dialog
                .w(px(520.))
                .title(t!("codex_enhance.enable_dialog_title").to_string())
                .child(
                    v_flex()
                        .w_full()
                        .gap(px(12.))
                        .child(
                            div()
                                .text_size(px(13.))
                                .text_color(cx.theme().muted_foreground)
                                .child(t!("codex_enhance.enable_dialog_message").to_string()),
                        )
                        .child(
                            h_flex().w_full().items_center().gap(px(8.)).child(
                                Checkbox::new("unify-migrate-existing")
                                    .label(t!("codex_enhance.migrate_existing").to_string())
                                    .on_click(move |checked: &bool, _window, cx| {
                                        let _ = view_checkbox.update(cx, |this, cx| {
                                            this.unify_dialog_migrate = *checked;
                                            cx.notify();
                                        });
                                    }),
                            ),
                        ),
                )
                .button_props(
                    DialogButtonProps::default()
                        .ok_text(t!("common.confirm").to_string())
                        .cancel_text(t!("common.cancel").to_string()),
                )
                .on_ok(move |_, _window, cx| {
                    let view = view_ok.clone();
                    let _ = view.update(cx, |this, cx| {
                        let migrate = this.unify_dialog_migrate;
                        match this
                            .workspace
                            .set_unify_codex_session_history(true, migrate)
                        {
                            Ok(outcome) => {
                                this.unify_codex_history = true;
                                let msg = unify_outcome_message(&outcome, true);
                                if outcome.is_skipped() {
                                    this.logs.push(msg.clone());
                                } else {
                                    notify_success(msg, _window, cx);
                                }
                                cx.notify();
                            }
                            Err(err) => {
                                this.fail(err, _window, cx);
                            }
                        }
                    });
                    true
                })
        });
    }

    fn open_unify_disable_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let view = cx.entity().downgrade();
        window.open_dialog(cx, move |dialog, _window, cx| {
            let view = view.clone();
            let view_checkbox = view.clone();
            let view_ok = view.clone();

            dialog
                .w(px(520.))
                .title(t!("codex_enhance.disable_dialog_title").to_string())
                .child(
                    v_flex()
                        .w_full()
                        .gap(px(12.))
                        .child(
                            div()
                                .text_size(px(13.))
                                .text_color(cx.theme().muted_foreground)
                                .child(t!("codex_enhance.disable_dialog_message").to_string()),
                        )
                        .child(
                            Checkbox::new("unify-restore-backup")
                                .label(t!("codex_enhance.restore_backup").to_string())
                                .on_click(move |checked: &bool, _window, cx| {
                                    let _ = view_checkbox.update(cx, |this, cx| {
                                        this.unify_dialog_restore = *checked;
                                        cx.notify();
                                    });
                                }),
                        ),
                )
                .button_props(
                    DialogButtonProps::default()
                        .ok_text(t!("common.confirm").to_string())
                        .cancel_text(t!("common.cancel").to_string()),
                )
                .on_ok(move |_, window, cx| {
                    let view = view_ok.clone();
                    let _ = view.update(cx, |this, cx| {
                        let restore = this.unify_dialog_restore;
                        // 关闭保存失败时绝不还原: 开关仍开着而账本已翻回，
                        // 会把历史拆成两半
                        if let Err(err) =
                            this.workspace.set_unify_codex_session_history(false, false)
                        {
                            this.fail(err, window, cx);
                            return;
                        }
                        this.unify_codex_history = false;
                        this.unify_dialog_restore = false;
                        if restore {
                            match this.workspace.restore_codex_unified_history() {
                                Ok(outcome) => {
                                    let msg = unify_outcome_message(&outcome, false);
                                    if outcome.is_skipped() {
                                        notify_info(msg, window, cx);
                                    } else {
                                        notify_success(msg, window, cx);
                                    }
                                }
                                Err(err) => {
                                    this.fail(err, window, cx);
                                }
                            }
                        }
                        cx.notify();
                    });
                    true
                })
        });
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
                        AppKind::ClaudeDesktop => {
                            "已切到 Claude Desktop 官方配置。可直接使用官方 Claude Desktop 登录。"
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
                        AppKind::ClaudeDesktop => format!("已启用 {} 并写入 Claude Desktop 3P 配置，请重启 Claude Desktop 生效。", provider_name),
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
                                let _ = this.workspace.delete_usage_script(&target);
                                this.usage_badges.remove(&target);
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
        form.website_url.update(cx, |input, cx| {
            input.set_value(preset.website_url, window, cx)
        });
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

    fn toggle_desktop_role_one_m(&mut self, idx: usize, cx: &mut Context<Self>) {
        if let Some(form) = self.form.as_mut() {
            if let Some(role) = form.desktop_roles.get_mut(idx) {
                role.one_m = !role.one_m;
                cx.notify();
            }
        }
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
                                        // Claude Desktop 映射角色同样挂上下拉
                                        for role in &mut form.desktop_roles {
                                            role.set_fetched_models(&models, window, cx);
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
        let claude_desktop_count = self.providers_for(AppKind::ClaudeDesktop).len();
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
                    Route::ClaudeDesktop,
                    Some(format!("{claude_desktop_count}")),
                    false,
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
                "nav-skills",
                IconName::Asterisk,
                Some(rgb(0x6366F1).into()), // Indigo
                t!("nav.skills").to_string(),
                Route::Skills,
                None,
                false,
                cx,
            ))
            .child(self.nav_item(
                "nav-prompts",
                CustomIcon::BookOpen,
                Some(rgb(0x10B981).into()), // Emerald
                t!("nav.prompts").to_string(),
                Route::Prompts,
                None,
                false,
                cx,
            ))
            .child(self.nav_item(
                "nav-sessions",
                CustomIcon::History,
                Some(rgb(0x8B5CF6).into()), // Violet
                t!("nav.sessions").to_string(),
                Route::Sessions,
                None,
                false,
                cx,
            ))
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

    fn render_usage_window_chip(
        &self,
        choice: UsageWindowChoice,
        label: String,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme().clone();
        let selected = self.usage_window == choice;
        div()
            .id(SharedString::from(format!("usage-window-{:?}", choice)))
            .h(px(26.))
            .px(px(12.))
            .rounded(px(13.))
            .flex()
            .items_center()
            .cursor_pointer()
            .text_size(px(12.))
            .font_weight(FontWeight::MEDIUM)
            .when(selected, |this| {
                this.bg(rgb(0x4F46E5)).text_color(rgb(0xFFFFFF))
            })
            .when(!selected, |this| {
                this.text_color(theme.muted_foreground)
                    .hover(|this| this.bg(theme.secondary.opacity(0.6)))
            })
            .child(label)
            .on_click(cx.listener(move |this, _, _, cx| {
                this.usage_window = choice;
                this.refresh_dashboard_data(cx);
            }))
    }

    fn render_usage_line_chart(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let default_data = DashboardUsageData::default();
        let data = self.dashboard_data.as_ref().unwrap_or(&default_data);

        // Rolling hour windows plot a dense HH:MM timeline; day windows plot days.
        let time_mode = !data.time_points.is_empty();
        let chart_points: Vec<(String, f64, i64)> = if time_mode {
            data.time_points
                .iter()
                .map(|p| (p.label.clone(), p.cost, p.tokens))
                .collect()
        } else {
            data.daily_points
                .iter()
                .map(|p| (p.label.clone(), p.cost, p.tokens))
                .collect()
        };
        let metric = self.usage_metric;
        let max_cost = if time_mode {
            data.max_time_cost
        } else {
            data.max_daily_cost
        };
        let max_tokens = if time_mode {
            data.max_time_tokens
        } else {
            data.max_daily_tokens
        };

        // Y-axis tick labels: quarters of the scaled peak
        let y_tick_label = |v: f64| {
            if metric == UsageMetric::Cost {
                format_currency(v)
            } else {
                format_tokens(v as i64)
            }
        };
        let max_val = if metric == UsageMetric::Cost {
            (max_cost * 1.25).max(1.0)
        } else {
            ((max_tokens as f64) * 1.25).max(1000.0)
        };
        let y_labels = [
            y_tick_label(max_val),
            y_tick_label(max_val * 0.75),
            y_tick_label(max_val * 0.5),
            y_tick_label(max_val * 0.25),
            if metric == UsageMetric::Cost {
                "$0.00".to_string()
            } else {
                "0".to_string()
            },
        ];

        // X-axis labels: ~6 evenly spaced ticks, first & last inclusive
        let x_labels: Vec<String> = if chart_points.is_empty() {
            vec!["—".to_string()]
        } else if chart_points.len() == 1 {
            vec![chart_points[0].0.clone()]
        } else if time_mode {
            let n = chart_points.len();
            let mut labels: Vec<String> = Vec::new();
            for i in [0usize, n / 5, 2 * n / 5, 3 * n / 5, 4 * n / 5, n - 1] {
                let label = chart_points[i].0.clone();
                if labels.last() != Some(&label) {
                    labels.push(label);
                }
            }
            labels
        } else if chart_points.len() <= 3 {
            chart_points.iter().map(|p| p.0.clone()).collect()
        } else {
            let n = chart_points.len();
            vec![
                chart_points[0].0.clone(),
                chart_points[n / 2].0.clone(),
                chart_points[n - 1].0.clone(),
            ]
        };

        let stroke_color = rgb(0x4F46E5);

        theme::tile(cx).w_full().child(
            v_flex()
                .w_full()
                .gap(px(12.))
                // 1. Chart Top Bar (Title + [费用 | Tokens] Switcher + Legend)
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
                                .child(if time_mode {
                                    if metric == UsageMetric::Cost {
                                        t!("usage.trend_cost").to_string()
                                    } else {
                                        t!("usage.trend_tokens").to_string()
                                    }
                                } else if metric == UsageMetric::Cost {
                                    t!("usage.daily_cost").to_string()
                                } else {
                                    t!("usage.daily_tokens").to_string()
                                }),
                        )
                        .child(
                            h_flex()
                                .items_center()
                                .gap(px(16.))
                                // Segmented Switcher [ Tokens | 费用 ]
                                .child(
                                    h_flex()
                                        .p(px(2.))
                                        .rounded(px(6.))
                                        .bg(theme.secondary.opacity(0.5))
                                        .border_1()
                                        .border_color(theme.border)
                                        .gap(px(2.))
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
                                        )
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
                                .children(y_labels.iter().map(|label| div().child(label.clone()))),
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
                                            div().w_full().h(px(1.)).bg(theme.border.opacity(0.45)),
                                        )
                                        .child(div().w_full().h(px(1.)).bg(theme.border)),
                                )
                                // Dynamic Vector Spline Curve Canvas
                                .child(
                                    gpui::canvas(
                                        |_bounds, _window, _cx| (),
                                        move |bounds, _state, window, _cx| {
                                            if chart_points.is_empty() {
                                                return;
                                            }

                                            let n = chart_points.len();
                                            let top_pad = 10.0;
                                            let bot_pad = 10.0;
                                            let width_f32 = f32::from(bounds.size.width);
                                            let height_f32 = f32::from(bounds.size.height);
                                            let draw_h = (height_f32 - top_pad - bot_pad).max(10.0);

                                            let mut pts: Vec<gpui::Point<gpui::Pixels>> =
                                                Vec::with_capacity(n);
                                            for (i, pt) in chart_points.iter().enumerate() {
                                                let x_norm = if n > 1 {
                                                    i as f32 / (n - 1) as f32
                                                } else {
                                                    0.5
                                                };
                                                let x =
                                                    bounds.origin.x + gpui::px(x_norm * width_f32);
                                                let val = if metric == UsageMetric::Cost {
                                                    pt.1
                                                } else {
                                                    pt.2 as f64
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

                                            // Point markers only for sparse (daily) series
                                            if pts.len() <= 40 {
                                                for p in &pts {
                                                    window.paint_quad(gpui::fill(
                                                        gpui::Bounds::new(
                                                            *p - gpui::point(
                                                                gpui::px(2.5),
                                                                gpui::px(2.5),
                                                            ),
                                                            gpui::size(
                                                                gpui::px(5.0),
                                                                gpui::px(5.0),
                                                            ),
                                                        ),
                                                        stroke_color,
                                                    ));
                                                }
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
                    .justify_end()
                    .gap(px(10.))
                    .p(px(8.))
                    .rounded(px(10.))
                    .bg(theme.secondary.opacity(0.35))
                    .border_1()
                    .border_color(theme.border)
                    .child(
                        // Time window: segmented control
                        h_flex()
                            .p(px(2.))
                            .rounded(px(8.))
                            .bg(theme.background)
                            .border_1()
                            .border_color(theme.border)
                            .gap(px(2.))
                            .children(
                                [
                                    (UsageWindowChoice::Hours6, t!("usage.hours_6").to_string()),
                                    (UsageWindowChoice::Hours24, t!("usage.hours_24").to_string()),
                                    (UsageWindowChoice::Days7, t!("usage.days_7").to_string()),
                                    (UsageWindowChoice::Days30, t!("usage.days_30").to_string()),
                                ]
                                .into_iter()
                                .map(|(choice, label)| {
                                    self.render_usage_window_chip(choice, label, cx)
                                }),
                            ),
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

    /// Codex 应用增强: 对齐 cc-switch 的两个增强开关，展示在 Codex 页最上方
    fn render_codex_enhancements(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

        v_flex()
            .w_full()
            .gap(px(10.))
            .child(
                theme::tile(cx).w_full().p(px(16.)).child(
                    h_flex()
                        .w_full()
                        .items_center()
                        .gap(px(12.))
                        .child(
                            div()
                                .size(px(36.))
                                .rounded(px(10.))
                                .bg(hsla(152. / 360., 0.76, 0.44, 1.0).opacity(0.12))
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    Icon::new(CustomIcon::KeyRound)
                                        .size(px(17.))
                                        .text_color(hsla(152. / 360., 0.76, 0.44, 1.0)),
                                ),
                        )
                        .child(
                            v_flex()
                                .flex_1()
                                .min_w_0()
                                .gap(px(4.))
                                .child(
                                    div()
                                        .text_size(px(13.5))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(theme.foreground)
                                        .child(t!("codex_enhance.preserve_title").to_string()),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .line_height(px(18.))
                                        .text_color(theme.muted_foreground)
                                        .child(
                                            t!("codex_enhance.preserve_description").to_string(),
                                        ),
                                ),
                        )
                        .child(
                            div()
                                .id("switch-preserve-codex-auth")
                                .cursor_pointer()
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.toggle_preserve_codex_auth(window, cx);
                                }))
                                .child(self.render_switch(self.preserve_codex_auth, cx)),
                        ),
                ),
            )
            .child({
                let sky = hsla(199. / 360., 0.89, 0.48, 1.0);
                let tile_id = SharedString::from("switch-unify-codex-history");
                let title = t!("codex_enhance.unify_title").to_string();
                let description = t!("codex_enhance.unify_description").to_string();
                theme::tile(cx).w_full().p(px(16.)).child(
                    h_flex()
                        .w_full()
                        .items_center()
                        .gap(px(12.))
                        .child(
                            div()
                                .size(px(36.))
                                .rounded(px(10.))
                                .bg(sky.opacity(0.12))
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    Icon::new(CustomIcon::History).size(px(17.)).text_color(sky),
                                ),
                        )
                        .child(
                            h_flex()
                                .flex_1()
                                .min_w_0()
                                .items_center()
                                .gap(px(6.))
                                .child(
                                    div()
                                        .text_size(px(13.5))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(theme.foreground)
                                        .child(title),
                                )
                                .child(
                                    div()
                                        .id("unify-codex-history-help")
                                        .cursor_pointer()
                                        .hoverable_tooltip(move |window, cx| {
                                            // 显式宽度保证长文本换行(flex 容器的
                                            // max_w 会被文本 min-content 撑破)
                                            let description = description.clone();
                                            Tooltip::element(move |_window, _cx| {
                                                div()
                                                    .w(px(360.))
                                                    .text_size(px(12.))
                                                    .line_height(px(19.))
                                                    .child(description.clone())
                                            })
                                            .build(window, cx)
                                        })
                                        .child(
                                            Icon::new(CustomIcon::HelpCircle)
                                                .size(px(13.))
                                                .text_color(theme.muted_foreground),
                                        ),
                                ),
                        )
                        .child(
                            div()
                                .id(tile_id)
                                .cursor_pointer()
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.toggle_unify_codex_history(window, cx);
                                }))
                                .child(self.render_switch(self.unify_codex_history, cx)),
                        ),
                )
            })
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
            .when(app == AppKind::Codex, |this| {
                this.child(self.render_codex_enhancements(cx))
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
                        h_flex()
                            .items_center()
                            .gap(px(8.))
                            .child(
                                Button::new(SharedString::from(format!(
                                    "{}-prompts-top",
                                    app.as_str()
                                )))
                                .outline()
                                .icon(CustomIcon::BookOpen)
                                .tooltip(t!("prompts.manage").to_string())
                                .on_click(cx.listener(
                                    move |this, _, _window, cx| {
                                        this.prompts_app = app;
                                        this.set_route(Route::Prompts, cx);
                                    },
                                )),
                            )
                            .child(
                                Button::new(SharedString::from(format!(
                                    "{}-sessions-top",
                                    app.as_str()
                                )))
                                .outline()
                                .icon(CustomIcon::History)
                                .tooltip(t!("nav.sessions").to_string())
                                .on_click(cx.listener(
                                    move |this, _, _window, cx| {
                                        this.sessions_filter = None;
                                        this.set_route(Route::Sessions, cx);
                                    },
                                )),
                            )
                            .when(app == AppKind::ClaudeDesktop, |this| {
                                this.child(
                                    Button::new("claude-desktop-import-top")
                                        .primary()
                                        .icon(CustomIcon::Download)
                                        .label(t!("claude_desktop.import_from_claude").to_string())
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.import_claude_desktop_from_claude_code(window, cx);
                                        })),
                                )
                            })
                            .child(
                                Button::new(SharedString::from(format!(
                                    "{}-add-top",
                                    app.as_str()
                                )))
                                .primary()
                                .icon(IconName::Plus)
                                .label(t!("provider.new").to_string())
                                .on_click(cx.listener(
                                    move |this, _, window, cx| {
                                        this.open_create_form(app, window, cx);
                                    },
                                )),
                            ),
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

    fn render_auth_card(
        &self,
        id: &'static str,
        icon: CustomIcon,
        icon_color: Hsla,
        title: &'static str,
        subtitle: &'static str,
        status_label: &'static str,
        login_label: String,
        status: Option<&session::NativeAuthStatus>,
        provider: &'static str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme().clone();
        let authenticated = status.is_some_and(|status| status.authenticated);
        let account = status.and_then(|status| status.account.clone());
        let pending = self
            .oauth_pending
            .as_ref()
            .is_some_and(|login| login.provider == provider);

        theme::tile(cx).child(
            v_flex()
                .w_full()
                .gap(px(12.))
                .child(
                    h_flex()
                        .items_center()
                        .gap(px(10.))
                        .child(
                            div()
                                .size(px(36.))
                                .rounded(px(10.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .bg(theme.secondary.opacity(0.6))
                                .child(Icon::new(icon).size(px(20.)).text_color(icon_color)),
                        )
                        .child(
                            v_flex()
                                .gap(px(2.))
                                .child(
                                    div()
                                        .text_size(px(14.))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(theme.foreground)
                                        .child(title),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .text_color(theme.muted_foreground)
                                        .child(subtitle),
                                ),
                        ),
                )
                .child(
                    h_flex()
                        .w_full()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_size(px(13.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(theme.foreground)
                                .child(status_label),
                        )
                        .child(
                            h_flex()
                                .items_center()
                                .gap(px(6.))
                                .children(account.clone().map(|account| {
                                    div()
                                        .text_size(px(12.))
                                        .text_color(theme.muted_foreground)
                                        .child(account)
                                }))
                                .child(if authenticated {
                                    Tag::success()
                                        .small()
                                        .child(t!("auth.status_ok").to_string())
                                } else {
                                    Tag::secondary()
                                        .small()
                                        .child(t!("auth.status_none").to_string())
                                }),
                        ),
                )
                .child(
                    Button::new(id)
                        .outline()
                        .w_full()
                        .icon(CustomIcon::ChartCurve)
                        .label(if pending {
                            t!("auth.waiting").to_string()
                        } else {
                            login_label
                        })
                        .disabled(pending)
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.start_oauth_login(provider, window, cx);
                        })),
                ),
        )
    }

    fn render_auth_settings(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap(px(16.))
            .child(self.render_auth_card(
                "auth-codex",
                CustomIcon::OpenAI,
                rgb(0x10A37F).into(),
                "ChatGPT (Codex OAuth)",
                t!("auth.codex_subtitle").to_string().leak(),
                t!("auth.codex_status").to_string().leak(),
                t!("auth.login_codex").to_string(),
                self.codex_oauth_status.as_ref(),
                session::CODEX_PROVIDER,
                cx,
            ))
            .child(self.render_auth_card(
                "auth-xai",
                CustomIcon::Grok,
                rgb(0x8B5CF6).into(),
                "xAI (Grok OAuth)",
                t!("auth.xai_subtitle").to_string().leak(),
                t!("auth.xai_status").to_string().leak(),
                t!("auth.login_xai").to_string(),
                self.xai_oauth_status.as_ref(),
                session::XAI_PROVIDER,
                cx,
            ))
    }

    /// 切换 Codex 映射行的思考档位声明
    fn toggle_catalog_row_level(
        &mut self,
        row_index: usize,
        level: &'static str,
        cx: &mut Context<Self>,
    ) {
        if let Some(form) = self.form.as_mut() {
            if form.app != AppKind::Codex {
                return;
            }
            if let Some(row) = form.catalog_rows.get_mut(row_index) {
                let mut levels = row
                    .reasoning_levels
                    .clone()
                    .unwrap_or_else(|| vec!["low".into(), "medium".into(), "high".into()]);
                if levels.iter().any(|l| l == level) {
                    levels.retain(|l| l != level);
                } else {
                    levels.push(level.to_string());
                }
                row.reasoning_levels = (!levels.is_empty()).then_some(levels);
                cx.notify();
            }
        }
    }

    fn refresh_skills(&mut self, cx: &mut Context<Self>) {
        self.skills_loading = true;
        cx.notify();
        let roots = self.workspace.skills_roots();
        let hub = self.workspace.skills_hub_dir();
        cx.spawn(
            move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
                let mut cx = cx.clone();
                async move {
                    let (skills, managed, unmanaged) = session::tokio_runtime()
                        .spawn(async move {
                            let skills = session::skills::scan_skills(&roots);
                            let managed = session::skills::hub::managed_dir_names(&hub);
                            let unmanaged: Vec<SkillsUnmanaged> =
                                session::skills::hub::scan_unmanaged(&roots, &hub)
                                    .into_iter()
                                    .map(|(app, path, dir_name, name, description)| {
                                        SkillsUnmanaged {
                                            app,
                                            path,
                                            dir_name,
                                            name,
                                            description,
                                        }
                                    })
                                    .collect();
                            (skills, managed, unmanaged)
                        })
                        .await
                        .unwrap_or_default();
                    let _ = this.update(&mut cx, |this, cx| {
                        this.skills_list = skills;
                        this.skills_managed_names = managed;
                        this.skills_unmanaged = unmanaged;
                        this.skills_loading = false;
                        cx.notify();
                    });
                }
            },
        )
        .detach();
    }

    /// 把 Skill 安装到目标应用(从任一已有副本拷贝)或从该应用移除
    fn toggle_skill_install(
        &mut self,
        entry: session::skills::SkillEntry,
        target_app: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let existing = entry
            .installs
            .iter()
            .find(|install| install.app == target_app);
        let is_managed = self
            .skills_managed_names
            .iter()
            .any(|name| name == &entry.dir_name);
        match existing {
            Some(install) => {
                // 非受管 Skill: 仅当其它应用仍有副本时才允许移除;
                // 受管 Skill 中心库保留主副本, 移除前先备份
                if !is_managed && entry.installs.len() <= 1 {
                    notify_info(t!("skills.last_copy").to_string(), window, cx);
                    return;
                }
                let backup_dir = self.workspace.skills_backup_dir();
                let path = install.path.clone();
                let remove_result = {
                    let backup = session::skills::hub::create_backup(&path, &backup_dir, "remove");
                    match backup {
                        Ok(_) => self.workspace.remove_skill(&path),
                        Err(err) => Err(err),
                    }
                };
                match remove_result {
                    Ok(()) => {
                        notify_success(t!("skills.removed").to_string(), window, cx);
                        self.refresh_skills(cx);
                    }
                    Err(err) => window.push_notification(Notification::error(err), cx),
                }
            }
            None => {
                let Some(source) = entry.installs.first().map(|install| install.path.clone())
                else {
                    return;
                };
                match self.workspace.install_skill(&source, target_app) {
                    Ok(()) => {
                        notify_success(t!("skills.installed").to_string(), window, cx);
                        self.refresh_skills(cx);
                    }
                    Err(err) => window.push_notification(Notification::error(err), cx),
                }
            }
        }
    }

    /// Skills 页工具栏(对齐 cc-switch): 检查更新 / 从备份中恢复 / 从 ZIP 安装 /
    /// 导入已有(有未纳管技能时带绿点) / 发现技能
    fn render_skills_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let checking = self.skills_checking_updates;
        let has_unmanaged = !self.skills_unmanaged.is_empty();
        h_flex()
            .items_center()
            .gap(px(2.))
            .child(
                Button::new("skills-check-updates")
                    .ghost()
                    .small()
                    .icon(if checking {
                        Icon::new(IconName::LoaderCircle)
                    } else {
                        Icon::new(CustomIcon::RotateCw)
                    })
                    .label(if checking {
                        t!("skills.checking_updates").to_string()
                    } else {
                        t!("skills.check_updates").to_string()
                    })
                    .disabled(checking || self.skills_managed_names.is_empty())
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.check_skills_updates(window, cx);
                    })),
            )
            .child(
                Button::new("skills-restore-backup")
                    .ghost()
                    .small()
                    .icon(IconName::Undo)
                    .label(t!("skills.restore_backup").to_string())
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.open_backups_dialog(window, cx);
                    })),
            )
            .child(
                Button::new("skills-install-zip")
                    .ghost()
                    .small()
                    .icon(IconName::Folder)
                    .label(t!("skills.install_zip").to_string())
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.install_skills_zip(window, cx);
                    })),
            )
            .child(
                div()
                    .relative()
                    .child(
                        Button::new("skills-import-existing")
                            .ghost()
                            .small()
                            .icon(IconName::ArrowDown)
                            .label(t!("skills.import_existing").to_string())
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_import_dialog(window, cx);
                            })),
                    )
                    .when(has_unmanaged, |this| {
                        this.child(
                            div()
                                .absolute()
                                .top(px(4.))
                                .right(px(4.))
                                .size(px(7.))
                                .rounded_full()
                                .bg(rgb(0x22C55E)),
                        )
                    }),
            )
            .child(
                Button::new("skills-discover")
                    .ghost()
                    .small()
                    .icon(IconName::Search)
                    .label(t!("skills.discover").to_string())
                    .on_click(cx.listener(|this, _, _window, cx| {
                        this.enter_skills_discover(cx);
                    })),
            )
    }

    /// 发现技能页(仓库 / skills.sh)
    fn render_skills_discover_page(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let search_value = self.skills_discover_search.read(cx).value().to_lowercase();
        let search = search_value.trim().to_lowercase();
        let source = self.skills_discover_source;
        let filter = self.skills_discover_filter;
        let loading = match source {
            SkillsDiscoverSource::Repos => self.skills_discover_loading,
            SkillsDiscoverSource::SkillsSh => self.skills_skills_sh_loading,
        };

        let visible: Vec<session::skills::hub::DiscoverableSkill> = {
            let all: Vec<session::skills::hub::DiscoverableSkill> = match source {
                SkillsDiscoverSource::Repos => self.skills_discover_list.clone(),
                SkillsDiscoverSource::SkillsSh => self.skills_skills_sh_list.clone(),
            };
            all.into_iter()
                .filter(|skill| {
                    let installed = self.is_skill_managed(&skill.directory);
                    match filter {
                        SkillsDiscoverFilter::All => true,
                        SkillsDiscoverFilter::Installed => installed,
                        SkillsDiscoverFilter::Uninstalled => !installed,
                    }
                })
                .filter(|skill| {
                    search.is_empty()
                        || skill.name.to_lowercase().contains(&search)
                        || skill.description.to_lowercase().contains(&search)
                        || format!("{}/{}", skill.repo_owner, skill.repo_name)
                            .to_lowercase()
                            .contains(&search)
                })
                .collect()
        };

        let source_toggle = h_flex()
            .items_center()
            .gap(px(4.))
            .p(px(3.))
            .rounded(px(8.))
            .border_1()
            .border_color(theme.border)
            .child(
                Button::new("skills-source-repos")
                    .xsmall()
                    .selected(source == SkillsDiscoverSource::Repos)
                    .label(t!("skills.source_repos").to_string())
                    .on_click(cx.listener(|this, _, _window, cx| {
                        this.set_skills_discover_source(SkillsDiscoverSource::Repos, cx);
                    })),
            )
            .child(
                Button::new("skills-source-skillssh")
                    .xsmall()
                    .selected(source == SkillsDiscoverSource::SkillsSh)
                    .label("skills.sh")
                    .on_click(cx.listener(|this, _, _window, cx| {
                        this.set_skills_discover_source(SkillsDiscoverSource::SkillsSh, cx);
                    })),
            );

        let filter_toggle = h_flex()
            .items_center()
            .gap(px(4.))
            .child(
                Button::new("skills-filter-all")
                    .ghost()
                    .xsmall()
                    .selected(filter == SkillsDiscoverFilter::All)
                    .label(t!("skills.filter_all").to_string())
                    .on_click(cx.listener(|this, _, _window, cx| {
                        this.skills_discover_filter = SkillsDiscoverFilter::All;
                        cx.notify();
                    })),
            )
            .child(
                Button::new("skills-filter-installed")
                    .ghost()
                    .xsmall()
                    .selected(filter == SkillsDiscoverFilter::Installed)
                    .label(t!("skills.filter_installed").to_string())
                    .on_click(cx.listener(|this, _, _window, cx| {
                        this.skills_discover_filter = SkillsDiscoverFilter::Installed;
                        cx.notify();
                    })),
            )
            .child(
                Button::new("skills-filter-uninstalled")
                    .ghost()
                    .xsmall()
                    .selected(filter == SkillsDiscoverFilter::Uninstalled)
                    .label(t!("skills.filter_uninstalled").to_string())
                    .on_click(cx.listener(|this, _, _window, cx| {
                        this.skills_discover_filter = SkillsDiscoverFilter::Uninstalled;
                        cx.notify();
                    })),
            );

        let mut controls = h_flex().items_center().gap(px(8.)).child(source_toggle);
        controls = controls.child(
            div()
                .flex_1()
                .min_w_0()
                .child(Input::new(&self.skills_discover_search)),
        );
        if source == SkillsDiscoverSource::SkillsSh {
            controls = controls.child(
                Button::new("skills-sh-search")
                    .primary()
                    .small()
                    .icon(IconName::Search)
                    .label(t!("skills.search_btn").to_string())
                    .disabled(self.skills_skills_sh_loading)
                    .on_click(cx.listener(|this, _, _window, cx| {
                        this.search_skills_sh(cx);
                    })),
            );
        }
        if source == SkillsDiscoverSource::Repos {
            controls = controls.child(filter_toggle);
        }

        let mut page = v_flex()
            .w_full()
            .p(px(24.))
            .gap(px(16.))
            // Header
            .child(
                h_flex()
                    .items_center()
                    .gap(px(12.))
                    .child(
                        Button::new("skills-discover-back")
                            .ghost()
                            .icon(IconName::ArrowLeft)
                            .on_click(cx.listener(|this, _, _window, cx| {
                                this.skills_view = SkillsView::Installed;
                                this.refresh_skills(cx);
                            })),
                    )
                    .child(
                        div()
                            .text_size(px(18.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.foreground)
                            .child(t!("skills.discover_title").to_string()),
                    )
                    .child(div().flex_1())
                    .child(
                        Button::new("skills-repo-manager")
                            .ghost()
                            .small()
                            .icon(IconName::Settings)
                            .label(t!("skills.repo_manager").to_string())
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_repo_manager_dialog(window, cx);
                            })),
                    )
                    .child(
                        Button::new("skills-discover-refresh")
                            .ghost()
                            .small()
                            .icon(if loading {
                                Icon::new(IconName::LoaderCircle)
                            } else {
                                Icon::new(CustomIcon::RotateCw)
                            })
                            .label(t!("sessions.refresh").to_string())
                            .disabled(loading)
                            .on_click(cx.listener(|this, _, _window, cx| {
                                match this.skills_discover_source {
                                    SkillsDiscoverSource::Repos => this.refresh_discover_skills(cx),
                                    SkillsDiscoverSource::SkillsSh => this.search_skills_sh(cx),
                                }
                            })),
                    ),
            )
            .child(controls);

        // 列表
        if loading && visible.is_empty() {
            page = page.child(
                div()
                    .w_full()
                    .py(px(48.))
                    .flex()
                    .justify_center()
                    .text_color(theme.muted_foreground)
                    .child(t!("skills.discover_loading").to_string()),
            );
        } else if visible.is_empty() {
            let hint = match source {
                SkillsDiscoverSource::Repos => {
                    if self.skills_discover_list.is_empty() {
                        t!("skills.discover_empty_repos").to_string()
                    } else {
                        t!("skills.no_results").to_string()
                    }
                }
                SkillsDiscoverSource::SkillsSh => {
                    if self.skills_skills_sh_query.is_empty() {
                        t!("skills.skillssh_hint").to_string()
                    } else {
                        t!("skills.no_results").to_string()
                    }
                }
            };
            page = page.child(
                div()
                    .w_full()
                    .py(px(48.))
                    .flex()
                    .justify_center()
                    .text_color(theme.muted_foreground)
                    .child(hint),
            );
        } else {
            let rows: Vec<_> = visible
                .iter()
                .enumerate()
                .map(|(idx, skill)| self.render_discover_row(skill, idx, cx).into_any_element())
                .collect();
            page = page.child(
                v_flex()
                    .id("skills-discover-list")
                    .w_full()
                    .max_h(px(640.))
                    .overflow_y_scroll()
                    .gap(px(8.))
                    .children(rows),
            );
        }

        // skills.sh 底部: 加载更多 + 署名
        if source == SkillsDiscoverSource::SkillsSh && !self.skills_skills_sh_list.is_empty() {
            let has_more = self.skills_skills_sh_list.len() < self.skills_skills_sh_total;
            page = page.child(
                v_flex()
                    .items_center()
                    .gap(px(6.))
                    .children(has_more.then(|| {
                        Button::new("skills-sh-more")
                            .outline()
                            .small()
                            .label(t!("skills.load_more").to_string())
                            .disabled(self.skills_skills_sh_loading)
                            .on_click(cx.listener(|this, _, _window, cx| {
                                this.load_more_skills_sh(cx);
                            }))
                    }))
                    .child(
                        div()
                            .text_size(px(11.5))
                            .text_color(theme.muted_foreground)
                            .child(t!("skills.powered_by").to_string()),
                    ),
            );
        }
        page
    }

    fn render_discover_row(
        &self,
        skill: &session::skills::hub::DiscoverableSkill,
        idx: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme().clone();
        let installed = self.is_skill_managed(&skill.directory);
        let installing = self
            .skills_installing_key
            .as_deref()
            .is_some_and(|key| key == skill.key);

        h_flex()
            .w_full()
            .items_center()
            .gap(px(12.))
            .p(px(12.))
            .rounded(px(10.))
            .border_1()
            .border_color(theme.border)
            .bg(theme.background)
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap(px(2.))
                    .child(
                        h_flex()
                            .items_center()
                            .gap(px(8.))
                            .child(
                                div()
                                    .text_size(px(13.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(theme.foreground)
                                    .child(skill.name.clone()),
                            )
                            .child(
                                Tag::secondary()
                                    .small()
                                    .child(format!("{}/{}", skill.repo_owner, skill.repo_name)),
                            )
                            .children(
                                skill
                                    .installs
                                    .map(|n| Tag::secondary().small().child(format!("⬇ {n}"))),
                            )
                            .children(installed.then(|| {
                                Tag::secondary()
                                    .small()
                                    .child(t!("skills.badge_installed").to_string())
                            })),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme.muted_foreground)
                            .max_w_full()
                            .truncate()
                            .child(if skill.description.is_empty() {
                                skill.directory.clone()
                            } else {
                                skill.description.clone()
                            }),
                    ),
            )
            .child(if installed {
                Button::new(SharedString::from(format!("skill-installed-{idx}")))
                    .outline()
                    .xsmall()
                    .label(t!("skills.badge_installed").to_string())
                    .disabled(true)
                    .into_any_element()
            } else {
                Button::new(SharedString::from(format!("skill-install-{idx}")))
                    .primary()
                    .xsmall()
                    .label(if installing {
                        t!("skills.installing").to_string()
                    } else {
                        t!("skills.install_action").to_string()
                    })
                    .disabled(installing)
                    .on_click(cx.listener({
                        let skill = skill.clone();
                        move |this, _, window, cx| {
                            this.install_discovered_skill(skill.clone(), window, cx);
                        }
                    }))
                    .into_any_element()
            })
    }

    fn is_skill_managed(&self, directory: &str) -> bool {
        let name = directory
            .rsplit('/')
            .next()
            .unwrap_or(directory)
            .to_lowercase();
        self.skills_managed_names
            .iter()
            .any(|managed| managed.to_lowercase() == name)
    }

    /// 进入发现技能页: 读取仓库配置, 首次自动发现
    fn enter_skills_discover(&mut self, cx: &mut Context<Self>) {
        self.skills_view = SkillsView::Discover;
        self.skills_repos = session::skills::hub::load_repos(&self.workspace.skills_hub_dir());
        cx.notify();
        if self.skills_discover_source == SkillsDiscoverSource::Repos
            && self.skills_discover_list.is_empty()
        {
            self.refresh_discover_skills(cx);
        }
    }

    fn set_skills_discover_source(&mut self, source: SkillsDiscoverSource, cx: &mut Context<Self>) {
        if self.skills_discover_source == source {
            return;
        }
        self.skills_discover_source = source;
        cx.notify();
        match source {
            SkillsDiscoverSource::Repos => {
                if self.skills_discover_list.is_empty() {
                    self.refresh_discover_skills(cx);
                }
            }
            SkillsDiscoverSource::SkillsSh => {}
        }
    }

    /// 从全部启用仓库拉取可发现技能(后台下载归档并扫描)
    fn refresh_discover_skills(&mut self, cx: &mut Context<Self>) {
        self.skills_repos = session::skills::hub::load_repos(&self.workspace.skills_hub_dir());
        if self.skills_discover_loading {
            return;
        }
        let repos = self.skills_repos.clone();
        self.skills_discover_loading = true;
        cx.notify();
        cx.spawn(
            move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
                let mut cx = cx.clone();
                async move {
                    let skills = session::tokio_runtime()
                        .spawn(async move {
                            let client = session::skills::hub::http_client();
                            session::skills::hub::discover_available(&client, &repos).await
                        })
                        .await
                        .unwrap_or_default();
                    let _ = this.update(&mut cx, |this, cx| {
                        this.skills_discover_list = skills;
                        this.skills_discover_loading = false;
                        cx.notify();
                    });
                }
            },
        )
        .detach();
    }

    /// 提交 skills.sh 搜索
    fn search_skills_sh(&mut self, cx: &mut Context<Self>) {
        let query = self
            .skills_discover_search
            .read(cx)
            .value()
            .trim()
            .to_string();
        if query.chars().count() < 2 {
            return;
        }
        if self.skills_skills_sh_loading {
            return;
        }
        self.skills_skills_sh_query = query.clone();
        self.skills_skills_sh_offset = 0;
        self.skills_skills_sh_loading = true;
        cx.notify();
        cx.spawn(
            move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
                let mut cx = cx.clone();
                async move {
                    let result = session::tokio_runtime()
                        .spawn(async move {
                            let client = session::skills::hub::http_client();
                            session::skills::hub::search_skills_sh(&client, &query, 20, 0).await
                        })
                        .await
                        .unwrap_or_else(|e| Err(format!("任务失败: {e}")));
                    let _ = this.update(&mut cx, |this, cx| {
                        this.skills_skills_sh_loading = false;
                        match result {
                            Ok(result) => {
                                this.skills_skills_sh_total = result.total_count;
                                this.skills_skills_sh_list = result.skills;
                            }
                            Err(err) => this.last_error = Some(err.into()),
                        }
                        cx.notify();
                    });
                }
            },
        )
        .detach();
    }

    fn load_more_skills_sh(&mut self, cx: &mut Context<Self>) {
        if self.skills_skills_sh_loading {
            return;
        }
        let query = self.skills_skills_sh_query.clone();
        let offset = self.skills_skills_sh_offset + 20;
        self.skills_skills_sh_offset = offset;
        self.skills_skills_sh_loading = true;
        cx.notify();
        cx.spawn(
            move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
                let mut cx = cx.clone();
                async move {
                    let result = session::tokio_runtime()
                        .spawn(async move {
                            let client = session::skills::hub::http_client();
                            session::skills::hub::search_skills_sh(&client, &query, 20, offset)
                                .await
                        })
                        .await
                        .unwrap_or_else(|e| Err(format!("任务失败: {e}")));
                    let _ = this.update(&mut cx, |this, cx| {
                        this.skills_skills_sh_loading = false;
                        match result {
                            Ok(result) => {
                                this.skills_skills_sh_total = result.total_count;
                                let known: std::collections::HashSet<String> = this
                                    .skills_skills_sh_list
                                    .iter()
                                    .map(|s| s.key.clone())
                                    .collect();
                                this.skills_skills_sh_list.extend(
                                    result
                                        .skills
                                        .into_iter()
                                        .filter(|s| !known.contains(&s.key)),
                                );
                            }
                            Err(err) => this.last_error = Some(err.into()),
                        }
                        cx.notify();
                    });
                }
            },
        )
        .detach();
    }

    /// 安装一个发现的技能: 中心库 + 全部应用目录
    fn install_discovered_skill(
        &mut self,
        skill: session::skills::hub::DiscoverableSkill,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.skills_installing_key.is_some() {
            return;
        }
        self.skills_installing_key = Some(skill.key.clone());
        cx.notify();
        let hub = self.workspace.skills_hub_dir();
        let backup_dir = self.workspace.skills_backup_dir();
        let app_dirs = self.workspace.skills_roots();
        let view = cx.entity().downgrade();
        window
            .spawn(cx, move |cx: &mut gpui::AsyncWindowContext| {
                let mut cx = cx.clone();
                async move {
                    let result = session::tokio_runtime()
                        .spawn(async move {
                            let client = session::skills::hub::http_client();
                            session::skills::hub::install_discovered(
                                &client,
                                &skill,
                                &hub,
                                &app_dirs,
                                &backup_dir,
                            )
                            .await
                        })
                        .await
                        .unwrap_or_else(|e| Err(format!("任务失败: {e}")));
                    let outcome = view.update(&mut cx, |this, cx| {
                        this.skills_installing_key = None;
                        let outcome = match result {
                            Ok(synced) => {
                                this.refresh_skills(cx);
                                Ok(synced)
                            }
                            Err(err) => Err(err),
                        };
                        cx.notify();
                        outcome
                    });
                    let _ = cx.update(|window: &mut Window, cx: &mut App| match outcome {
                        Ok(Ok(synced)) => window.push_notification(
                            Notification::success(
                                t!("skills.install_done", count = synced).to_string(),
                            ),
                            cx,
                        ),
                        Ok(Err(err)) => window.push_notification(Notification::error(err), cx),
                        Err(err) => {
                            window.push_notification(Notification::error(err.to_string()), cx)
                        }
                    });
                }
            })
            .detach();
    }

    /// 检查全部受管技能的更新, 有更新时弹出列表对话框
    fn check_skills_updates(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.skills_checking_updates {
            return;
        }
        self.skills_checking_updates = true;
        cx.notify();
        let hub = self.workspace.skills_hub_dir();
        let view = cx.entity().downgrade();
        window
            .spawn(cx, move |cx: &mut gpui::AsyncWindowContext| {
                let mut cx = cx.clone();
                async move {
                    let updates = session::tokio_runtime()
                        .spawn(async move {
                            let client = session::skills::hub::http_client();
                            session::skills::hub::check_updates(&client, &hub).await
                        })
                        .await
                        .unwrap_or_else(|e| Err(format!("任务失败: {e}")));
                    let outcome = view.update(&mut cx, |this, cx| {
                        this.skills_checking_updates = false;
                        cx.notify();
                        updates
                    });
                    let _ = cx.update(|window: &mut Window, cx: &mut App| match outcome {
                        Ok(Ok(list)) if list.is_empty() => window.push_notification(
                            Notification::success(t!("skills.up_to_date").to_string()),
                            cx,
                        ),
                        Ok(Ok(list)) => {
                            let on_update = {
                                let view = view.clone();
                                move |dir_name: String, window: &mut Window, cx: &mut App| {
                                    let _ = view.update(cx, |this, cx| {
                                        this.update_managed_skill(dir_name, window, cx);
                                    });
                                }
                            };
                            open_skills_updates_dialog(window, cx, list, on_update);
                        }
                        Ok(Err(err)) => window.push_notification(Notification::error(err), cx),
                        Err(err) => {
                            window.push_notification(Notification::error(err.to_string()), cx)
                        }
                    });
                }
            })
            .detach();
    }

    /// 更新单个受管技能(重新下载并同步)
    fn update_managed_skill(
        &mut self,
        dir_name: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let hub = self.workspace.skills_hub_dir();
        let backup_dir = self.workspace.skills_backup_dir();
        let app_dirs = self.workspace.skills_roots();
        window
            .spawn(cx, move |cx: &mut gpui::AsyncWindowContext| {
                let mut cx = cx.clone();
                async move {
                    let result = session::tokio_runtime()
                        .spawn(async move {
                            let client = session::skills::hub::http_client();
                            session::skills::hub::update_skill(
                                &client,
                                &dir_name,
                                &hub,
                                &app_dirs,
                                &backup_dir,
                            )
                            .await
                        })
                        .await
                        .unwrap_or_else(|e| Err(format!("任务失败: {e}")));
                    let _ = cx.update(|window: &mut Window, cx: &mut App| match result {
                        Ok(synced) => window.push_notification(
                            Notification::success(
                                t!("skills.update_done", count = synced).to_string(),
                            ),
                            cx,
                        ),
                        Err(err) => window.push_notification(Notification::error(err), cx),
                    });
                }
            })
            .detach();
    }

    /// 备份列表对话框
    fn open_backups_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let backup_dir = self.workspace.skills_backup_dir();
        let backups = session::skills::hub::list_backups(&backup_dir);
        if backups.is_empty() {
            notify_info(t!("skills.backup_empty").to_string(), window, cx);
            return;
        }
        let view = cx.entity().downgrade();
        window.open_dialog(cx, move |dialog, _window, cx| {
            let view = view.clone();
            let rows: Vec<_> = backups
                .iter()
                .map(|entry| render_backup_row(entry, view.clone(), cx).into_any_element())
                .collect();
            dialog
                .width(px(640.))
                .title(t!("skills.backup_title").to_string())
                .child(
                    v_flex()
                        .id("skills-backup-list")
                        .w_full()
                        .max_h(px(420.))
                        .overflow_y_scroll()
                        .gap(px(8.))
                        .children(rows),
                )
        });
    }

    fn restore_skill_backup(
        &mut self,
        entry: session::skills::hub::SkillBackupEntry,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let backup_dir = self.workspace.skills_backup_dir();
        let hub = self.workspace.skills_hub_dir();
        match session::skills::hub::restore_backup(&entry, &backup_dir, &hub) {
            Ok(()) => {
                window.close_dialog(cx);
                notify_success(t!("skills.backup_restored").to_string(), window, cx);
                self.refresh_skills(cx);
            }
            Err(err) => window.push_notification(Notification::error(err), cx),
        }
    }

    fn delete_skill_backup(
        &mut self,
        backup_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let backup_dir = self.workspace.skills_backup_dir();
        match session::skills::hub::delete_backup(&backup_dir, &backup_id) {
            Ok(()) => {
                window.close_dialog(cx);
                notify_success(t!("skills.backup_deleted").to_string(), window, cx);
            }
            Err(err) => window.push_notification(Notification::error(err), cx),
        }
    }

    /// 删除备份前的确认对话框
    fn confirm_delete_backup(
        &mut self,
        backup_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let view = cx.entity().downgrade();
        window.open_dialog(cx, move |dialog, _window, cx| {
            let view = view.clone();
            let id = backup_id.clone();
            dialog
                .width(px(420.))
                .title(t!("skills.backup_delete_title").to_string())
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(cx.theme().muted_foreground)
                        .child(t!("skills.backup_delete_confirm").to_string()),
                )
                .footer(move |_ok, _cancel, _window, _cx| {
                    let view = view.clone();
                    let id = id.clone();
                    vec![h_flex()
                        .w_full()
                        .justify_end()
                        .gap(px(8.))
                        .child(
                            Button::new("backup-delete-cancel")
                                .ghost()
                                .small()
                                .label(t!("skills.cancel").to_string())
                                .on_click(|_, window, cx| {
                                    window.close_dialog(cx);
                                }),
                        )
                        .child(
                            Button::new("backup-delete-confirm")
                                .primary()
                                .small()
                                .label(t!("skills.backup_delete").to_string())
                                .on_click(move |_, window, cx| {
                                    window.close_dialog(cx);
                                    let _ = view.update(cx, |this, cx| {
                                        this.delete_skill_backup(id.clone(), window, cx)
                                    });
                                }),
                        )
                        .into_any_element()]
                })
        });
    }

    /// 导入已有(未纳管)技能对话框
    fn open_import_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.skills_unmanaged.is_empty() {
            notify_info(t!("skills.import_empty").to_string(), window, cx);
            return;
        }
        self.skills_import_selected = self
            .skills_unmanaged
            .iter()
            .map(|s| s.dir_name.clone())
            .collect();
        let view = cx.entity().downgrade();
        let unmanaged = self.skills_unmanaged.clone();
        let selected = self.skills_import_selected.clone();
        window.open_dialog(cx, move |dialog, _window, cx| {
            let view = view.clone();
            let selected = selected.clone();
            let rows: Vec<_> = unmanaged
                .iter()
                .map(|item| {
                    let is_selected = selected.contains(&item.dir_name);
                    render_import_row(item, is_selected, view.clone(), cx).into_any_element()
                })
                .collect();
            let confirm_view = view.clone();
            dialog
                .width(px(600.))
                .title(t!("skills.import_title").to_string())
                .child(
                    v_flex()
                        .id("skills-import-list")
                        .w_full()
                        .max_h(px(420.))
                        .overflow_y_scroll()
                        .gap(px(8.))
                        .children(rows),
                )
                .footer(move |_ok, _cancel, _window, _cx| {
                    let confirm_view = confirm_view.clone();
                    vec![h_flex()
                        .w_full()
                        .justify_end()
                        .items_center()
                        .gap(px(8.))
                        .child(
                            Button::new("skills-import-cancel")
                                .ghost()
                                .small()
                                .label(t!("skills.cancel").to_string())
                                .on_click(|_, window, cx| {
                                    window.close_dialog(cx);
                                }),
                        )
                        .child(
                            Button::new("skills-import-confirm")
                                .primary()
                                .small()
                                .label(t!("skills.import_confirm").to_string())
                                .on_click(move |_, window, cx| {
                                    let v = confirm_view.clone();
                                    window.close_dialog(cx);
                                    let _ = v.update(cx, |this, cx| {
                                        this.import_selected_skills(window, cx);
                                    });
                                }),
                        )
                        .into_any_element()]
                })
        });
    }

    /// 把勾选的未纳管技能收编进中心库
    fn import_selected_skills(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let selected = self.skills_import_selected.clone();
        let hub = self.workspace.skills_hub_dir();
        let backup_dir = self.workspace.skills_backup_dir();
        let items: Vec<SkillsUnmanaged> = self
            .skills_unmanaged
            .iter()
            .filter(|s| selected.contains(&s.dir_name))
            .cloned()
            .collect();
        let mut ok = 0usize;
        let mut first_error = None;
        for item in &items {
            match session::skills::hub::import_to_hub(&item.path, &hub, &backup_dir) {
                Ok(_) => ok += 1,
                Err(err) => {
                    first_error.get_or_insert(err);
                }
            }
        }
        if let Some(err) = first_error {
            window.push_notification(Notification::error(err), cx);
        } else {
            notify_success(t!("skills.import_done", count = ok).to_string(), window, cx);
        }
        self.refresh_skills(cx);
    }

    /// 从 ZIP 安装: 选择文件 → 后台解压发现 → 入库并同步
    fn install_skills_zip(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(t!("skills.zip_pick").to_string().into()),
        });
        let view = cx.entity().downgrade();
        window
            .spawn(cx, move |cx: &mut gpui::AsyncWindowContext| {
                let mut cx = cx.clone();
                async move {
                    let Ok(Ok(Some(paths))) = receiver.await else {
                        return;
                    };
                    let Some(path) = paths.first().cloned() else {
                        return;
                    };
                    let too_large = std::fs::metadata(&path)
                        .map(|m| m.len() > session::skills::hub::MAX_DOWNLOAD_BYTES)
                        .unwrap_or(true);
                    if too_large {
                        let _ = cx.update(|window: &mut Window, cx: &mut App| {
                            window.push_notification(
                                Notification::error(t!("skills.zip_too_large").to_string()),
                                cx,
                            );
                        });
                        return;
                    }
                    let (hub, backup_dir, app_dirs) = view
                        .update(&mut cx, |this, _| {
                            (
                                this.workspace.skills_hub_dir(),
                                this.workspace.skills_backup_dir(),
                                this.workspace.skills_roots(),
                            )
                        })
                        .unwrap_or_default();
                    let result: Result<Vec<String>, String> = cx
                        .background_executor()
                        .spawn(async move {
                            std::fs::read(&path)
                                .map_err(|e| format!("读取 ZIP 失败: {e}"))
                                .and_then(|bytes| {
                                    session::skills::hub::install_from_zip_bytes(
                                        bytes,
                                        &hub,
                                        &app_dirs,
                                        &backup_dir,
                                    )
                                })
                        })
                        .await;
                    let outcome = view.update(&mut cx, |this, cx| {
                        let outcome = match result {
                            Ok(names) => {
                                this.refresh_skills(cx);
                                Ok(names)
                            }
                            Err(err) => Err(err),
                        };
                        cx.notify();
                        outcome
                    });
                    let _ = cx.update(|window: &mut Window, cx: &mut App| match outcome {
                        Ok(Ok(names)) => window.push_notification(
                            Notification::success(
                                t!("skills.zip_done", count = names.len()).to_string(),
                            ),
                            cx,
                        ),
                        Ok(Err(err)) => window.push_notification(Notification::error(err), cx),
                        Err(err) => {
                            window.push_notification(Notification::error(err.to_string()), cx)
                        }
                    });
                }
            })
            .detach();
    }

    /// 仓库管理对话框: 启用/删除 + 新增
    fn open_repo_manager_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.skills_repos = session::skills::hub::load_repos(&self.workspace.skills_hub_dir());
        let view = cx.entity().downgrade();
        let owner_input = self.skills_repo_owner.clone();
        let name_input = self.skills_repo_name.clone();
        let branch_input = self.skills_repo_branch.clone();
        let repos = self.skills_repos.clone();
        let theme = cx.theme().clone();
        window.open_dialog(cx, move |dialog, _window, cx| {
            let view = view.clone();
            let (owner_input, name_input, branch_input) = (
                owner_input.clone(),
                name_input.clone(),
                branch_input.clone(),
            );
            let rows: Vec<_> = repos
                .iter()
                .map(|repo| render_repo_row(repo, view.clone(), cx).into_any_element())
                .collect();
            let v_add = view.clone();
            dialog
                .width(px(620.))
                .title(t!("skills.repo_title").to_string())
                .child(
                    v_flex()
                        .w_full()
                        .gap(px(10.))
                        .children(rows)
                        .child(
                            h_flex()
                                .items_center()
                                .gap(px(8.))
                                .child(div().w(px(150.)).child(Input::new(&owner_input)))
                                .child(div().w(px(210.)).child(Input::new(&name_input)))
                                .child(div().w(px(110.)).child(Input::new(&branch_input)))
                                .child(
                                    Button::new("skills-repo-add")
                                        .outline()
                                        .small()
                                        .icon(IconName::Plus)
                                        .label(t!("skills.repo_add").to_string())
                                        .on_click(move |_, window, cx| {
                                            let owner =
                                                owner_input.read(cx).value().trim().to_string();
                                            let name =
                                                name_input.read(cx).value().trim().to_string();
                                            let branch =
                                                branch_input.read(cx).value().trim().to_string();
                                            let view = v_add.clone();
                                            let _ = view.update(cx, |this, cx| {
                                                this.add_skill_repo(owner, name, branch, window, cx)
                                            });
                                        }),
                                ),
                        )
                        .child(
                            div()
                                .text_size(px(11.5))
                                .text_color(theme.muted_foreground)
                                .child(t!("skills.repo_hint").to_string()),
                        ),
                )
        });
    }

    fn add_skill_repo(
        &mut self,
        owner: String,
        name: String,
        branch: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let branch = if branch.is_empty() {
            "main".to_string()
        } else {
            branch
        };
        if let Err(err) = session::skills::hub::validate_repo_ref(&owner, &name, &branch) {
            window.push_notification(Notification::error(err), cx);
            return;
        }
        if self
            .skills_repos
            .iter()
            .any(|r| r.owner.eq_ignore_ascii_case(&owner) && r.name.eq_ignore_ascii_case(&name))
        {
            notify_info(t!("skills.repo_exists").to_string(), window, cx);
            return;
        }
        self.skills_repos.push(session::skills::hub::SkillRepo {
            owner,
            name,
            branch,
            enabled: true,
        });
        if let Err(err) =
            session::skills::hub::save_repos(&self.workspace.skills_hub_dir(), &self.skills_repos)
        {
            window.push_notification(Notification::error(err), cx);
            return;
        }
        // 清空输入并刷新发现列表
        let owner_input = self.skills_repo_owner.clone();
        let name_input = self.skills_repo_name.clone();
        let branch_input = self.skills_repo_branch.clone();
        owner_input.update(cx, |state, cx| state.set_value("", window, cx));
        name_input.update(cx, |state, cx| state.set_value("", window, cx));
        branch_input.update(cx, |state, cx| state.set_value("", window, cx));
        self.refresh_discover_skills(cx);
    }

    fn remove_skill_repo(&mut self, owner: String, name: String, cx: &mut Context<Self>) {
        self.skills_repos
            .retain(|r| !(r.owner == owner && r.name == name));
        let _ =
            session::skills::hub::save_repos(&self.workspace.skills_hub_dir(), &self.skills_repos);
        self.refresh_discover_skills(cx);
    }

    fn toggle_skill_repo_enabled(
        &mut self,
        owner: String,
        name: String,
        enabled: bool,
        cx: &mut Context<Self>,
    ) {
        for repo in &mut self.skills_repos {
            if repo.owner == owner && repo.name == name {
                repo.enabled = enabled;
            }
        }
        let _ =
            session::skills::hub::save_repos(&self.workspace.skills_hub_dir(), &self.skills_repos);
        self.refresh_discover_skills(cx);
    }

    fn refresh_sessions(&mut self, cx: &mut Context<Self>) {
        self.sessions_loading = true;
        cx.notify();
        let (codex_roots, claude_root) = self.workspace.session_roots();
        cx.spawn(
            move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
                let mut cx = cx.clone();
                async move {
                    // 扫描文件系统, 放到后台线程
                    let sessions = session::tokio_runtime()
                        .spawn(async move {
                            session::sessions::scan_sessions(&codex_roots, &claude_root)
                        })
                        .await
                        .unwrap_or_default();
                    let _ = this.update(&mut cx, |this, cx| {
                        this.sessions_list = sessions;
                        this.sessions_loading = false;
                        this.session_checked.clear();
                        if let Some(selected) = &this.session_selected {
                            // 清掉已删除的选中项
                            if !this
                                .sessions_list
                                .iter()
                                .any(|s| s.source_path == selected.source_path)
                            {
                                this.session_selected = None;
                                this.session_messages.clear();
                            }
                        }
                        cx.notify();
                    });
                }
            },
        )
        .detach();
    }

    fn select_session(&mut self, meta: session::sessions::SessionMeta, cx: &mut Context<Self>) {
        self.session_selected = Some(meta.clone());
        self.session_messages.clear();
        self.session_messages_loading = true;
        cx.notify();

        let Some(source_path) = meta.source_path.clone() else {
            self.session_messages_loading = false;
            return;
        };
        let provider_id = meta.provider_id.clone();
        let (codex_roots, claude_root) = self.workspace.session_roots();
        cx.spawn(
            move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
                let mut cx = cx.clone();
                async move {
                    let messages = session::tokio_runtime()
                        .spawn(async move {
                            session::sessions::load_messages(
                                &provider_id,
                                &source_path,
                                &codex_roots,
                                &claude_root,
                            )
                            .unwrap_or_default()
                        })
                        .await
                        .unwrap_or_default();
                    let _ = this.update(&mut cx, |this, cx| {
                        this.session_messages = messages;
                        this.session_messages_loading = false;
                        cx.notify();
                    });
                }
            },
        )
        .detach();
    }

    fn toggle_session_checked(&mut self, source_path: &str, cx: &mut Context<Self>) {
        if self.session_checked.contains(source_path) {
            self.session_checked.remove(source_path);
        } else {
            self.session_checked.insert(source_path.to_string());
        }
        cx.notify();
    }

    fn delete_checked_sessions(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let checked = self.session_checked.clone();
        if checked.is_empty() {
            notify_info(t!("sessions.none_selected").to_string(), window, cx);
            return;
        }
        let requests: Vec<session::sessions::DeleteSessionRequest> = self
            .sessions_list
            .iter()
            .filter(|meta| checked.contains(meta.source_path.as_deref().unwrap_or_default()))
            .map(|meta| session::sessions::DeleteSessionRequest {
                provider_id: meta.provider_id.clone(),
                session_id: meta.session_id.clone(),
                source_path: meta.source_path.clone().unwrap_or_default(),
            })
            .collect();
        let outcomes = self.workspace.delete_sessions(&requests);
        let deleted = outcomes.iter().filter(|o| o.success).count();
        let failed = outcomes.len() - deleted;
        notify_success(
            t!("sessions.deleted", ok = deleted, fail = failed).to_string(),
            window,
            cx,
        );
        self.session_checked.clear();
        self.refresh_sessions(cx);
    }

    fn delete_selected_session(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(selected) = self.session_selected.clone() else {
            return;
        };
        let requests = vec![session::sessions::DeleteSessionRequest {
            provider_id: selected.provider_id.clone(),
            session_id: selected.session_id.clone(),
            source_path: selected.source_path.clone().unwrap_or_default(),
        }];
        let outcomes = self.workspace.delete_sessions(&requests);
        if outcomes.iter().all(|o| o.success) {
            notify_success(t!("sessions.deleted_one").to_string(), window, cx);
        } else if let Some(error) = outcomes.first().and_then(|o| o.error.clone()) {
            window.push_notification(Notification::error(error), cx);
        }
        self.session_selected = None;
        self.session_messages.clear();
        self.refresh_sessions(cx);
    }

    fn resume_selected_session(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(selected) = self.session_selected.clone() else {
            return;
        };
        let Some(command) = selected.resume_command.clone() else {
            return;
        };
        let dir = selected.project_dir.clone();
        // macOS: 用 Terminal.app 在项目目录打开并执行恢复命令
        let script = match &dir {
            Some(dir) if !dir.trim().is_empty() => format!(
                "tell application \"Terminal\"\nactivate\ndo script \"cd {} && {}\"\nend tell",
                dir.replace('"', "\\\""),
                command.replace('"', "\\\"")
            ),
            _ => format!(
                "tell application \"Terminal\"\nactivate\ndo script \"{}\"\nend tell",
                command.replace('"', "\\\"")
            ),
        };
        let output = std::process::Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .output();
        match output {
            Ok(_) => notify_success(t!("sessions.resumed").to_string(), window, cx),
            Err(_) => {
                // 兜底: 复制命令到剪贴板
                cx.write_to_clipboard(gpui::ClipboardItem::new_string(command));
                notify_info(t!("sessions.command_copied").to_string(), window, cx);
            }
        }
    }

    fn render_skills_page(&self, cx: &mut Context<Self>) -> AnyElement {
        if self.skills_view == SkillsView::Discover {
            return self.render_skills_discover_page(cx).into_any_element();
        }
        let theme = cx.theme().clone();
        let search_value = self.skills_search.read(cx).value().to_lowercase();
        let search = search_value.trim().to_lowercase();

        let visible: Vec<&session::skills::SkillEntry> = self
            .skills_list
            .iter()
            .filter(|entry| {
                search.is_empty()
                    || entry.dir_name.to_lowercase().contains(&search)
                    || entry
                        .name
                        .as_deref()
                        .unwrap_or_default()
                        .to_lowercase()
                        .contains(&search)
                    || entry
                        .description
                        .as_deref()
                        .unwrap_or_default()
                        .to_lowercase()
                        .contains(&search)
            })
            .collect();

        let app_meta: &[(&'static str, CustomIcon, Hsla, &'static str)] = &[
            (
                session::skills::APP_CLAUDE,
                CustomIcon::Claude,
                rgb(0xD97757).into(),
                "Claude",
            ),
            (
                session::skills::APP_CODEX,
                CustomIcon::OpenAI,
                rgb(0x10A37F).into(),
                "Codex",
            ),
            (
                session::skills::APP_GROK,
                CustomIcon::Grok,
                rgb(0x8B5CF6).into(),
                "Grok Build",
            ),
            (
                session::skills::APP_OPENCODE,
                CustomIcon::OpenCode,
                rgb(0x6366F1).into(),
                "OpenCode",
            ),
            (
                session::skills::APP_PI,
                CustomIcon::Pi,
                rgb(0x3B82F6).into(),
                "Pi",
            ),
        ];

        // 每应用计数
        let count_chips = app_meta.iter().map(|(app, icon, color, label)| {
            let count = self
                .skills_list
                .iter()
                .filter(|entry| entry.installs.iter().any(|install| install.app == *app))
                .count();
            Tag::secondary().small().child(
                h_flex()
                    .items_center()
                    .gap(px(4.))
                    .child(Icon::new(*icon).size(px(12.)).text_color(*color))
                    .child(
                        div()
                            .text_size(px(11.5))
                            .text_color(*color)
                            .child(format!("{label}: {count}")),
                    ),
            )
        });

        v_flex()
            .w_full()
            .p(px(24.))
            .gap(px(16.))
            // Header
            .child(
                h_flex()
                    .items_center()
                    .gap(px(12.))
                    .child(
                        Button::new("skills-back")
                            .ghost()
                            .icon(IconName::ArrowLeft)
                            .on_click(cx.listener(|this, _, _window, cx| {
                                this.set_route(Route::Dashboard, cx);
                            })),
                    )
                    .child(
                        div()
                            .text_size(px(18.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.foreground)
                            .child(t!("skills.title").to_string()),
                    )
                    .child(div().flex_1())
                    // 工具栏(对齐 cc-switch): 检查更新 / 从备份中恢复 / 从 ZIP 安装 /
                    // 导入已有(有未纳管技能时带绿点) / 发现技能
                    .child(self.render_skills_toolbar(cx))
                    .child(
                        Button::new("skills-refresh")
                            .ghost()
                            .icon(CustomIcon::RotateCw)
                            .tooltip(t!("sessions.refresh").to_string())
                            .disabled(self.skills_loading)
                            .on_click(cx.listener(|this, _, _window, cx| {
                                this.refresh_skills(cx);
                            })),
                    ),
            )
            // 统计条
            .child(
                theme::tile(cx).child(
                    h_flex()
                        .w_full()
                        .items_center()
                        .gap(px(8.))
                        .child(
                            Button::new("skills-installed-chip")
                                .outline()
                                .xsmall()
                                .selected(true)
                                .label(t!("skills.installed").to_string()),
                        )
                        .child(div().flex_1())
                        .children(count_chips),
                ),
            )
            // 搜索
            .child(Input::new(&self.skills_search))
            // Skill 列表
            .child(
                theme::tile(cx).p(px(0.)).overflow_hidden().child(
                    div()
                        .id("skills-list-scroll")
                        .max_h(px(640.))
                        .flex()
                        .flex_col()
                        .overflow_y_scroll()
                        .children(visible.iter().enumerate().map(|(index, entry)| {
                            let divider = index > 0;
                            let entry_for_actions = (*entry).clone();
                            self.render_skill_row(entry_for_actions, divider, app_meta, cx)
                        })),
                ),
            )
            .children(self.skills_loading.then(|| {
                div()
                    .text_size(px(12.))
                    .text_color(theme.muted_foreground)
                    .child(t!("sessions.loading").to_string())
            }))
            .when(visible.is_empty() && !self.skills_loading, |this| {
                this.child(
                    v_flex().items_center().py(px(48.)).child(
                        div()
                            .text_size(px(13.))
                            .text_color(theme.muted_foreground)
                            .child(t!("skills.empty").to_string()),
                    ),
                )
            })
            .into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn render_skill_row(
        &self,
        entry: session::skills::SkillEntry,
        divider: bool,
        app_meta: &[(&'static str, CustomIcon, Hsla, &'static str)],
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme().clone();
        let display_name = entry.name.clone().unwrap_or_else(|| entry.dir_name.clone());
        let description = entry.description.clone().unwrap_or_default();

        let app_icons = app_meta
            .iter()
            .map(|(app, icon, color, label)| {
                let installed = entry.installs.iter().any(|install| install.app == *app);
                let entry_for_click = entry.clone();
                let app_for_click = *app;
                let label_for_tooltip = *label;
                div()
                    .id(SharedString::from(format!(
                        "skill-app-{}-{}",
                        entry.dir_name, app
                    )))
                    .size(px(26.))
                    .rounded(px(13.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .bg(if installed {
                        color.opacity(0.15)
                    } else {
                        theme.secondary.opacity(0.35)
                    })
                    .tooltip(move |window, cx| {
                        let text = if installed {
                            format!("{label_for_tooltip}: 已安装，点击移除")
                        } else {
                            format!("{label_for_tooltip}: 未安装，点击安装")
                        };
                        gpui_component::tooltip::Tooltip::new(text).build(window, cx)
                    })
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.toggle_skill_install(
                            entry_for_click.clone(),
                            app_for_click,
                            window,
                            cx,
                        );
                    }))
                    .child(Icon::new(*icon).size(px(14.)).text_color(if installed {
                        *color
                    } else {
                        theme.muted_foreground.opacity(0.45)
                    }))
            })
            .collect::<Vec<_>>();

        div()
            .w_full()
            .px(px(16.))
            .py(px(10.))
            .when(divider, |this| {
                this.border_t_1().border_color(theme.border.opacity(0.6))
            })
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .gap(px(12.))
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .gap(px(3.))
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(theme.foreground)
                                            .child(display_name),
                                    )
                                    .child(
                                        Tag::secondary()
                                            .small()
                                            .child(t!("skills.local").to_string()),
                                    ),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .truncate()
                                    .text_size(px(12.))
                                    .text_color(theme.muted_foreground)
                                    .child(description),
                            ),
                    )
                    .child(h_flex().items_center().gap(px(6.)).children(app_icons)),
            )
    }

    fn render_sessions_page(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let search_value = self.sessions_search.read(cx).value().to_lowercase();
        let search = search_value.trim().to_lowercase();

        let visible: Vec<session::sessions::SessionMeta> = self
            .sessions_list
            .iter()
            .filter(|meta| {
                self.sessions_filter
                    .as_deref()
                    .is_none_or(|f| meta.provider_id == f)
            })
            .filter(|meta| {
                search.is_empty()
                    || meta
                        .title
                        .as_deref()
                        .unwrap_or_default()
                        .to_lowercase()
                        .contains(&search)
                    || meta
                        .project_dir
                        .as_deref()
                        .unwrap_or_default()
                        .to_lowercase()
                        .contains(&search)
                    || meta.session_id.contains(&search)
            })
            .cloned()
            .collect();

        let visible_rc = std::rc::Rc::new(visible);
        let visible_count = visible_rc.len();
        let visible_for_list = visible_rc.clone();

        v_flex()
            .size_full()
            .p(px(20.))
            .gap(px(16.))
            .overflow_hidden()
            // Header
            .child(
                h_flex()
                    .items_center()
                    .gap(px(12.))
                    .child(
                        Button::new("sessions-back")
                            .ghost()
                            .icon(IconName::ArrowLeft)
                            .on_click(cx.listener(|this, _, _window, cx| {
                                this.set_route(Route::Dashboard, cx);
                            })),
                    )
                    .child(
                        div()
                            .text_size(px(18.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.foreground)
                            .child(t!("sessions.title").to_string()),
                    ),
            )
            .child(
                h_flex()
                    .w_full()
                    .flex_1()
                    .min_h_0()
                    .gap(px(16.))
                    // 左栏: 会话列表 (450px 宽，独立滚动)
                    .child(
                        v_flex()
                            .w(px(450.))
                            .h_full()
                            .min_h_0()
                            .relative()
                            .gap(px(8.))
                            .child(
                                h_flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        h_flex()
                                            .items_center()
                                            .gap(px(6.))
                                            .child(
                                                div()
                                                    .text_size(px(14.))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(theme.foreground)
                                                    .child(t!("sessions.list").to_string()),
                                            )
                                            .child(
                                                Tag::secondary()
                                                    .small()
                                                    .child(format!("{}", visible_count)),
                                            ),
                                    )
                                    // 图标工具栏: 批量选择 / 筛选 / 搜索 / 刷新
                                    .child(
                                        h_flex()
                                            .items_center()
                                            .gap(px(2.))
                                            .child(
                                                Button::new("sessions-batch-toggle")
                                                    .ghost()
                                                    .xsmall()
                                                    .icon(IconName::Check)
                                                    .selected(self.sessions_batch_mode)
                                                    .tooltip(
                                                        t!("sessions.batch_tooltip").to_string(),
                                                    )
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.sessions_batch_mode =
                                                            !this.sessions_batch_mode;
                                                        if !this.sessions_batch_mode {
                                                            this.session_checked.clear();
                                                        }
                                                        cx.notify();
                                                    })),
                                            )
                                            .child(
                                                Button::new("sessions-filter-toggle")
                                                    .ghost()
                                                    .xsmall()
                                                    .icon({
                                                        let icon: Icon =
                                                            match self.sessions_filter.as_deref() {
                                                                Some("codex") => {
                                                                    CustomIcon::OpenAI.into()
                                                                }
                                                                Some("grok") => {
                                                                    CustomIcon::Grok.into()
                                                                }
                                                                Some("claude") => {
                                                                    CustomIcon::Claude.into()
                                                                }
                                                                Some("opencode") => {
                                                                    CustomIcon::OpenCode.into()
                                                                }
                                                                Some("openclaw") => {
                                                                    CustomIcon::OhMyPi.into()
                                                                }
                                                                Some("gemini") => {
                                                                    CustomIcon::Gemini.into()
                                                                }
                                                                Some("pi") => {
                                                                    CustomIcon::Pi.into()
                                                                }
                                                                Some("zcode") => {
                                                                    CustomIcon::ZCode.into()
                                                                }
                                                                Some("workbuddy") => {
                                                                    CustomIcon::WorkBuddy.into()
                                                                }
                                                                Some("cursor") => {
                                                                    CustomIcon::Cursor.into()
                                                                }
                                                                _ => IconName::Asterisk.into(),
                                                            };
                                                        icon
                                                    })
                                                    .tooltip(
                                                        t!("sessions.filter_tooltip").to_string(),
                                                    )
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.sessions_filter_menu_open =
                                                            !this.sessions_filter_menu_open;
                                                        cx.notify();
                                                    })),
                                            )
                                            .child(
                                                Button::new("sessions-search-toggle")
                                                    .ghost()
                                                    .xsmall()
                                                    .icon(IconName::Search)
                                                    .selected(self.sessions_search_open)
                                                    .tooltip(t!("sessions.search").to_string())
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.sessions_search_open =
                                                            !this.sessions_search_open;
                                                        cx.notify();
                                                    })),
                                            )
                                            .child(
                                                Button::new("sessions-refresh")
                                                    .ghost()
                                                    .xsmall()
                                                    .icon(CustomIcon::RotateCw)
                                                    .tooltip(t!("sessions.refresh").to_string())
                                                    .disabled(self.sessions_loading)
                                                    .on_click(cx.listener(
                                                        |this, _, _window, cx| {
                                                            this.refresh_sessions(cx);
                                                        },
                                                    )),
                                            ),
                                    ),
                            )
                            // 搜索行(点放大镜展开)
                            .when(self.sessions_search_open, |this| {
                                this.child(
                                    h_flex().gap(px(6.)).child(
                                        div().flex_1().child(Input::new(&self.sessions_search)),
                                    ),
                                )
                            })
                            // 批量操作面板(点勾选图标展开)
                            .when(self.sessions_batch_mode, |this| {
                                let checked_count = self.session_checked.len();
                                this.child(
                                    theme::tile(cx).child(
                                        v_flex()
                                            .w_full()
                                            .gap(px(8.))
                                            .child(
                                                h_flex()
                                                    .items_center()
                                                    .gap(px(6.))
                                                    .child(Tag::secondary().small().child(format!(
                                                        "{} {}",
                                                        t!("sessions.selected_prefix"),
                                                        checked_count
                                                    )))
                                                    .child(
                                                        div()
                                                            .text_size(px(11.5))
                                                            .text_color(theme.muted_foreground)
                                                            .child(
                                                                t!("sessions.select_hint")
                                                                    .to_string(),
                                                            ),
                                                    ),
                                            )
                                            .child(
                                                h_flex()
                                                    .items_center()
                                                    .gap(px(10.))
                                                    .child(
                                                        Button::new("sessions-select-all")
                                                            .ghost()
                                                            .xsmall()
                                                            .label(
                                                                t!("sessions.select_all")
                                                                    .to_string(),
                                                            )
                                                            .on_click(cx.listener(
                                                                |this, _, _, cx| {
                                                                    let visible: Vec<String> = this
                                                                        .sessions_list
                                                                        .iter()
                                                                        .filter(|meta| {
                                                                            this.sessions_filter
                                                                                .as_deref()
                                                                                .is_none_or(|f| {
                                                                                    meta.provider_id
                                                                                        == f
                                                                                })
                                                                        })
                                                                        .filter_map(|meta| {
                                                                            meta.source_path.clone()
                                                                        })
                                                                        .collect();
                                                                    for path in visible {
                                                                        this.session_checked
                                                                            .insert(path);
                                                                    }
                                                                    cx.notify();
                                                                },
                                                            )),
                                                    )
                                                    .child(
                                                        Button::new("sessions-clear-checked")
                                                            .ghost()
                                                            .xsmall()
                                                            .label(t!("sessions.clear").to_string())
                                                            .on_click(cx.listener(
                                                                |this, _, _, cx| {
                                                                    this.session_checked.clear();
                                                                    cx.notify();
                                                                },
                                                            )),
                                                    )
                                                    .child(
                                                        Button::new("sessions-batch-delete")
                                                            .outline()
                                                            .xsmall()
                                                            .icon(IconName::Delete)
                                                            .label(
                                                                t!("sessions.batch_delete")
                                                                    .to_string(),
                                                            )
                                                            .on_click(cx.listener(
                                                                |this, _, window, cx| {
                                                                    this.delete_checked_sessions(
                                                                        window, cx,
                                                                    );
                                                                },
                                                            )),
                                                    ),
                                            ),
                                    ),
                                )
                            })
                            // 虚拟化会话列表 (通过 uniform_list + cx.processor 杜绝 300+ 条目缩放与滚动卡顿)
                            .child(
                                div()
                                    .id("sessions-list-container")
                                    .flex_1()
                                    .min_h_0()
                                    .w_full()
                                    .child(if visible_count == 0 {
                                        v_flex()
                                            .items_center()
                                            .justify_center()
                                            .py(px(60.))
                                            .child(
                                                div()
                                                    .text_size(px(13.))
                                                    .text_color(theme.muted_foreground)
                                                    .child(t!("sessions.empty").to_string()),
                                            )
                                            .into_any_element()
                                    } else {
                                        uniform_list(
                                            "sessions-uniform-list",
                                            visible_count,
                                            cx.processor(move |this: &mut Self, range: std::ops::Range<usize>, _window: &mut Window, cx: &mut Context<Self>| {
                                                let theme = cx.theme().clone();
                                                let dark = theme.is_dark();
                                                let selected_path = this
                                                    .session_selected
                                                    .as_ref()
                                                    .and_then(|s| s.source_path.clone());
                                                let mut items = Vec::with_capacity(range.len());
                                                for ix in range {
                                                    if let Some(meta) = visible_for_list.get(ix) {
                                                        let source = meta.source_path.clone().unwrap_or_default();
                                                        let checked = this.session_checked.contains(&source);
                                                        let is_selected = selected_path == meta.source_path;
                                                        let (icon, icon_color) = match meta.provider_id.as_str() {
                                                            "codex" => (CustomIcon::OpenAI, rgb(0x10A37F)),
                                                            "grok" => (CustomIcon::Grok, rgb(0x8B5CF6)),
                                                            "claude" => (CustomIcon::Claude, rgb(0xD97757)),
                                                            "opencode" => (CustomIcon::OpenCode, rgb(0x0284C7)),
                                                            "openclaw" => (CustomIcon::OhMyPi, rgb(0xEC4899)),
                                                            "gemini" => (CustomIcon::Gemini, rgb(0x2563EB)),
                                                            "pi" => (CustomIcon::Pi, rgb(0x3B82F6)),
                                                            "zcode" => (CustomIcon::ZCode, rgb(0x3B82F6)),
                                                            "workbuddy" => (CustomIcon::WorkBuddy, rgb(0x6366F1)),
                                                            "cursor" => (CustomIcon::Cursor, if dark { rgb(0xFFFFFF) } else { rgb(0x000000) }),
                                                            _ => (CustomIcon::OpenAI, rgb(0x10A37F)),
                                                        };
                                                        let now_secs = std::time::SystemTime::now()
                                                            .duration_since(std::time::UNIX_EPOCH)
                                                            .map(|d| d.as_secs() as i64)
                                                            .unwrap_or(0);
                                                        let relative = relative_time_text(
                                                            now_secs
                                                                - meta
                                                                    .last_active_at
                                                                    .or(meta.created_at)
                                                                    .unwrap_or(0)
                                                                    / 1000,
                                                            this.language,
                                                        );
                                                        let title = meta
                                                            .title
                                                            .clone()
                                                            .unwrap_or_else(|| meta.session_id.clone());
                                                        let project_dir = meta.project_dir.as_deref().and_then(|d| {
                                                            let clean = d.lines().next().unwrap_or("").trim();
                                                            if clean.is_empty() {
                                                                None
                                                            } else {
                                                                Some(clean.to_string())
                                                            }
                                                        });
                                                        let source_toggle = source.clone();
                                                        let meta_click = meta.clone();

                                                        items.push(
                                                            div()
                                                                .id(SharedString::from(format!("session-item-{}", ix)))
                                                                .h(px(68.))
                                                                .py(px(3.))
                                                                .child(
                                                                    div()
                                                                        .id(SharedString::from(format!("session-card-{}", ix)))
                                                                        .w_full()
                                                                        .h_full()
                                                                        .p(px(10.))
                                                                        .rounded(px(8.))
                                                                        .border_1()
                                                                        .border_color(if is_selected {
                                                                            rgb(0x3B82F6).into()
                                                                        } else {
                                                                            theme.border.opacity(0.85)
                                                                        })
                                                                        .bg(if is_selected {
                                                                            if dark {
                                                                                gpui::Hsla::from(rgba(0x3B82F626))
                                                                            } else {
                                                                                gpui::Hsla::from(rgba(0x3B82F618))
                                                                            }
                                                                        } else {
                                                                            theme.background
                                                                        })
                                                                        .cursor_pointer()
                                                                        .overflow_hidden()
                                                                        .hover(|style| {
                                                                            if is_selected {
                                                                                style
                                                                            } else {
                                                                                style.bg(theme.secondary.opacity(0.5))
                                                                            }
                                                                        })
                                                                        .on_click(cx.listener(move |this, _, _, cx| {
                                                                            this.select_session(meta_click.clone(), cx);
                                                                        }))
                                                                        .child(
                                                                            v_flex()
                                                                                .w_full()
                                                                                .h_full()
                                                                                .justify_between()
                                                                                .overflow_hidden()
                                                                                .child(
                                                                                    h_flex()
                                                                                        .w_full()
                                                                                        .items_center()
                                                                                        .gap(px(8.))
                                                                                        .when(this.sessions_batch_mode, |parent| {
                                                                                            parent.child(
                                                                                                div()
                                                                                                    .id(SharedString::from(format!("chk-{}", source_toggle)))
                                                                                                    .size(px(16.))
                                                                                                    .rounded(px(4.))
                                                                                                    .border_1()
                                                                                                    .border_color(if checked {
                                                                                                        gpui::Hsla::from(rgb(0x2563EB))
                                                                                                    } else {
                                                                                                        theme.border
                                                                                                    })
                                                                                                    .bg(if checked {
                                                                                                        gpui::Hsla::from(rgb(0x2563EB))
                                                                                                    } else {
                                                                                                        gpui::transparent_black()
                                                                                                    })
                                                                                                    .flex()
                                                                                                    .items_center()
                                                                                                    .justify_center()
                                                                                                    .on_click(cx.listener(move |this, _, _, cx| {
                                                                                                        this.toggle_session_checked(&source_toggle, cx);
                                                                                                    }))
                                                                                                    .when(checked, |chk| {
                                                                                                        chk.child(
                                                                                                            Icon::new(IconName::Check)
                                                                                                                .size(px(10.))
                                                                                                                .text_color(rgb(0xFFFFFF)),
                                                                                                        )
                                                                                                    }),
                                                                                            )
                                                                                        })
                                                                                        .child(
                                                                                            Icon::new(icon)
                                                                                                .size(px(16.))
                                                                                                .text_color(icon_color),
                                                                                        )
                                                                                        .child(
                                                                                            div()
                                                                                                .flex_1()
                                                                                                .min_w_0()
                                                                                                .truncate()
                                                                                                .text_size(px(13.))
                                                                                                .font_weight(FontWeight::MEDIUM)
                                                                                                .text_color(theme.foreground)
                                                                                                .child(title),
                                                                                        )
                                                                                        .child(
                                                                                            Icon::new(IconName::ChevronRight)
                                                                                                .size(px(13.))
                                                                                                .text_color(theme.muted_foreground.opacity(0.6)),
                                                                                        ),
                                                                                )
                                                                                .child(
                                                                                    h_flex()
                                                                                        .w_full()
                                                                                        .items_center()
                                                                                        .gap(px(6.))
                                                                                        .overflow_hidden()
                                                                                        .child(
                                                                                            div()
                                                                                                .text_size(px(11.))
                                                                                                .text_color(theme.muted_foreground)
                                                                                                .child(relative),
                                                                                        )
                                                                                        .children(project_dir.map(|dir| {
                                                                                            div()
                                                                                                .flex_1()
                                                                                                .min_w_0()
                                                                                                .truncate()
                                                                                                .text_size(px(11.))
                                                                                                .text_color(theme.muted_foreground.opacity(0.75))
                                                                                                .child(dir)
                                                                                        })),
                                                                                ),
                                                                        ),
                                                                ),
                                                        );
                                                    }
                                                }
                                                items
                                            }),
                                        )
                                        .h_full()
                                        .into_any_element()
                                    }),
                            )
                            // 应用筛选下拉菜单 (在 DOM 末尾渲染以确保绘制在最上层，避免被列表遮挡)
                            .when(self.sessions_filter_menu_open, |this| {
                                let filter_options: [(Option<String>, &str, Option<CustomIcon>, Hsla); 10] = [
                                    (None, "全部", None, theme.foreground),
                                    (Some("codex".to_string()), "Codex", Some(CustomIcon::OpenAI), rgb(0x10A37F).into()),
                                    (Some("grok".to_string()), "Grok Build", Some(CustomIcon::Grok), rgb(0x8B5CF6).into()),
                                    (Some("claude".to_string()), "Claude Code", Some(CustomIcon::Claude), rgb(0xD97757).into()),
                                    (Some("opencode".to_string()), "OpenCode", Some(CustomIcon::OpenCode), rgb(0x0284C7).into()),
                                    (Some("openclaw".to_string()), "OpenClaw", Some(CustomIcon::OhMyPi), rgb(0xEC4899).into()),
                                    (Some("gemini".to_string()), "Gemini CLI", Some(CustomIcon::Gemini), rgb(0x2563EB).into()),
                                    (Some("pi".to_string()), "Pi", Some(CustomIcon::Pi), rgb(0x3B82F6).into()),
                                    (Some("zcode".to_string()), "ZCode", Some(CustomIcon::ZCode), rgb(0x3B82F6).into()),
                                    (Some("workbuddy".to_string()), "WorkBuddy", Some(CustomIcon::WorkBuddy), rgb(0x6366F1).into()),
                                ];
                                this.child(
                                    div()
                                        .absolute()
                                        .top(px(36.))
                                        .right(px(0.))
                                        .w(px(180.))
                                        .p(px(6.))
                                        .rounded(px(10.))
                                        .bg(theme.background)
                                        .border_1()
                                        .border_color(theme.border)
                                        .shadow_lg()
                                        .child(
                                            v_flex().gap(px(2.)).children(
                                                filter_options
                                                    .into_iter()
                                                    .map(|(filter, label, icon, icon_color)| {
                                                        let active = self.sessions_filter == filter;
                                                        div()
                                                            .id(SharedString::from(format!(
                                                                    "sessions-filter-menu-{:?}",
                                                                    filter
                                                                )))
                                                            .flex()
                                                            .items_center()
                                                            .gap(px(8.))
                                                            .px(px(8.))
                                                            .py(px(6.))
                                                            .rounded(px(6.))
                                                            .cursor_pointer()
                                                            .hover(|this| {
                                                                this.bg(theme.secondary.opacity(0.6))
                                                            })
                                                            .on_click(cx.listener(
                                                                move |this, _, _, cx| {
                                                                    this.sessions_filter =
                                                                        filter.clone();
                                                                    this.sessions_filter_menu_open =
                                                                        false;
                                                                    cx.notify();
                                                                },
                                                            ))
                                                            .child(
                                                                div()
                                                                    .size(px(14.))
                                                                    .flex()
                                                                    .items_center()
                                                                    .justify_center()
                                                                    .when(active, |chk| {
                                                                        chk.child(
                                                                            Icon::new(IconName::Check)
                                                                                .size(px(13.))
                                                                                .text_color(theme.foreground),
                                                                        )
                                                                    }),
                                                            )
                                                            .child(
                                                                div()
                                                                    .size(px(18.))
                                                                    .flex()
                                                                    .items_center()
                                                                    .justify_center()
                                                                    .child(if let Some(ic) = icon {
                                                                        Icon::new(ic)
                                                                            .size(px(16.))
                                                                            .text_color(icon_color)
                                                                            .into_any_element()
                                                                    } else {
                                                                        div()
                                                                            .size(px(16.))
                                                                            .rounded(px(4.))
                                                                            .bg(theme.secondary)
                                                                            .flex()
                                                                            .items_center()
                                                                            .justify_center()
                                                                            .text_size(px(10.))
                                                                            .font_weight(FontWeight::BOLD)
                                                                            .text_color(theme.foreground)
                                                                            .child("A")
                                                                            .into_any_element()
                                                                    }),
                                                            )
                                                            .child(
                                                                div()
                                                                    .flex_1()
                                                                    .text_size(px(13.))
                                                                    .font_weight(if active {
                                                                        FontWeight::SEMIBOLD
                                                                    } else {
                                                                        FontWeight::NORMAL
                                                                    })
                                                                    .text_color(theme.foreground)
                                                                    .child(label),
                                                            )
                                                    }),
                                            ),
                                        ),
                                )
                            }),
                    )
                    // 右栏: 会话详情 (自适应拓宽，独立滚动展示空间)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .h_full()
                            .min_h_0()
                            .p(px(16.))
                            .rounded(px(12.))
                            .border_1()
                            .border_color(theme.border)
                            .bg(theme.background)
                            .overflow_hidden()
                            .child(match self.session_selected.clone() {
                                Some(selected) => {
                                    self.render_session_detail(selected, cx).into_any_element()
                                }
                                None => v_flex()
                                    .size_full()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .text_color(theme.muted_foreground)
                                            .child(t!("sessions.pick_hint").to_string()),
                                    )
                                    .into_any_element(),
                            }),
                    ),
            )
    }

    fn render_session_detail(
        &self,
        selected: session::sessions::SessionMeta,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme().clone();
        let muted = theme.muted_foreground;
        let dark = theme.is_dark();
        let (icon, icon_color) = match selected.provider_id.as_str() {
            "codex" => (CustomIcon::OpenAI, rgb(0x10A37F)),
            "grok" => (CustomIcon::Grok, rgb(0x8B5CF6)),
            "claude" => (CustomIcon::Claude, rgb(0xD97757)),
            "opencode" => (CustomIcon::OpenCode, rgb(0x0284C7)),
            "openclaw" => (CustomIcon::OhMyPi, rgb(0xEC4899)),
            "gemini" => (CustomIcon::Gemini, rgb(0x2563EB)),
            "pi" => (CustomIcon::Pi, rgb(0x3B82F6)),
            "zcode" => (CustomIcon::ZCode, rgb(0x3B82F6)),
            "workbuddy" => (CustomIcon::WorkBuddy, rgb(0x6366F1)),
            "cursor" => (
                CustomIcon::Cursor,
                if dark { rgb(0xFFFFFF) } else { rgb(0x000000) },
            ),
            _ => (CustomIcon::OpenAI, rgb(0x10A37F)),
        };
        let title = selected
            .title
            .clone()
            .unwrap_or_else(|| selected.session_id.clone());
        let created = selected
            .created_at
            .map(|ms| {
                chrono::DateTime::<chrono::Local>::from(
                    std::time::UNIX_EPOCH + std::time::Duration::from_millis(ms as u64),
                )
                .format("%Y/%m/%d %H:%M:%S")
                .to_string()
            })
            .unwrap_or_default();
        let file_name = selected
            .source_path
            .as_deref()
            .and_then(|p| std::path::Path::new(p).file_name())
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default();
        let resume_command = selected.resume_command.clone().unwrap_or_default();
        let role_label = |role: &str| -> String {
            match role {
                "user" => t!("sessions.role_user").to_string(),
                "tool" => t!("sessions.role_tool").to_string(),
                other => other.to_string(),
            }
        };
        let role_color = |role: &str| -> Hsla {
            match role {
                "user" => rgb(0x2563EB).into(),
                "assistant" => rgb(0x10A37F).into(),
                "tool" => rgb(0xF59E0B).into(),
                _ => muted,
            }
        };

        v_flex()
            .size_full()
            .min_h_0()
            .gap(px(12.))
            // 标题行 + 操作
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .child(
                        h_flex()
                            .items_center()
                            .gap(px(8.))
                            .child(Icon::new(icon).size(px(20.)).text_color(icon_color))
                            .child(
                                div()
                                    .text_size(px(16.))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(theme.foreground)
                                    .child(title),
                            ),
                    )
                    .child(
                        h_flex()
                            .items_center()
                            .gap(px(8.))
                            .child(
                                Button::new("session-resume")
                                    .primary()
                                    .small()
                                    .label(t!("sessions.resume").to_string())
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.resume_selected_session(window, cx);
                                    })),
                            )
                            .child(
                                Button::new("session-delete")
                                    .outline()
                                    .small()
                                    .icon(IconName::Delete)
                                    .label(t!("sessions.delete_one").to_string())
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.delete_selected_session(window, cx);
                                    })),
                            ),
                    ),
            )
            // 元信息
            .child(
                h_flex()
                    .items_center()
                    .gap(px(14.))
                    .child(div().text_size(px(11.5)).text_color(muted).child(created))
                    .children(
                        selected
                            .project_dir
                            .clone()
                            .map(|dir| div().text_size(px(11.5)).text_color(muted).child(dir)),
                    )
                    .child(
                        div()
                            .text_size(px(11.5))
                            .text_color(muted)
                            .truncate()
                            .child(file_name),
                    ),
            )
            // 恢复命令
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .gap(px(8.))
                    .p(px(10.))
                    .rounded(px(8.))
                    .bg(theme.secondary.opacity(0.4))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .font_family("Menlo")
                            .text_size(px(12.))
                            .text_color(theme.foreground)
                            .child(resume_command.clone()),
                    )
                    .child(
                        Button::new("session-copy-cmd")
                            .ghost()
                            .xsmall()
                            .icon(IconName::Copy)
                            .on_click(cx.listener(move |_, _, _, cx| {
                                cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                    resume_command.clone(),
                                ));
                            })),
                    ),
            )
            // 对话记录
            .child(
                h_flex()
                    .items_center()
                    .gap(px(6.))
                    .child(
                        div()
                            .text_size(px(14.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.foreground)
                            .child(t!("sessions.messages").to_string()),
                    )
                    .child(
                        Tag::secondary()
                            .small()
                            .child(format!("{}", self.session_messages.len())),
                    )
                    .children(self.session_messages_loading.then(|| {
                        div()
                            .text_size(px(11.5))
                            .text_color(muted)
                            .child(t!("sessions.loading").to_string())
                    })),
            )
            .child(
                div()
                    .id("session-messages-scroll")
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .overflow_y_scroll()
                    .children(self.session_messages.iter().take(100).map(|message| {
                        let role = message.role.clone();
                        let content_for_copy = message.content.clone();
                        let content = message.content.clone();
                        let ts = message
                            .ts
                            .map(|ms| {
                                chrono::DateTime::<chrono::Local>::from(
                                    std::time::UNIX_EPOCH
                                        + std::time::Duration::from_millis(ms as u64),
                                )
                                .format("%Y/%m/%d %H:%M:%S")
                                .to_string()
                            })
                            .unwrap_or_default();
                        let color = role_color(&role);
                        div()
                            .w_full()
                            .p(px(14.))
                            .rounded(px(8.))
                            .border_1()
                            .border_color(theme.border)
                            .bg(theme.background)
                            .child(
                                v_flex()
                                    .w_full()
                                    .gap(px(8.))
                                    .child(
                                        h_flex()
                                            .w_full()
                                            .items_center()
                                            .justify_between()
                                            .child(
                                                h_flex()
                                                    .items_center()
                                                    .gap(px(8.))
                                                    .child(
                                                        div()
                                                            .text_size(px(12.))
                                                            .font_weight(FontWeight::SEMIBOLD)
                                                            .text_color(color)
                                                            .child(role_label(&role)),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_size(px(11.5))
                                                            .text_color(muted)
                                                            .child(ts),
                                                    ),
                                            )
                                            .child(
                                                Button::new(SharedString::from(format!(
                                                    "msg-copy-{}",
                                                    message.ts.unwrap_or(0)
                                                )))
                                                .ghost()
                                                .xsmall()
                                                .icon(IconName::Copy)
                                                .on_click(cx.listener(move |_, _, _, cx| {
                                                    cx.write_to_clipboard(
                                                        gpui::ClipboardItem::new_string(
                                                            content_for_copy.clone(),
                                                        ),
                                                    );
                                                })),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .w_full()
                                            .text_size(px(13.))
                                            .line_height(px(20.))
                                            .text_color(theme.foreground)
                                            .child(content),
                                    ),
                            )
                    })),
            )
    }

    fn refresh_prompts(&mut self, cx: &mut Context<Self>) {
        self.prompts_loading = true;
        let app = self.prompts_app;
        let _ = self.workspace.import_prompt_from_file_on_first_launch(app);
        match self.workspace.get_prompts(app) {
            Ok(list) => {
                self.prompts_list = list;
            }
            Err(e) => {
                eprintln!("[router-switch] 加载提示词失败: {e}");
            }
        }
        self.prompts_loading = false;
        cx.notify();
    }

    fn switch_prompts_app(&mut self, app: AppKind, cx: &mut Context<Self>) {
        self.prompts_app = app;
        self.refresh_prompts(cx);
    }

    fn toggle_prompt_enabled(
        &mut self,
        app: AppKind,
        id: &str,
        current_enabled: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let res = if current_enabled {
            self.workspace.disable_prompt(app, id)
        } else {
            self.workspace.enable_prompt(app, id)
        };
        match res {
            Ok(()) => {
                self.refresh_prompts(cx);
                if !current_enabled {
                    notify_success("已启用提示词并同步至本地配置文件", window, cx);
                } else {
                    notify_info("已停用提示词", window, cx);
                }
            }
            Err(e) => {
                notify_error(format!("切换提示词状态失败: {e}"), window, cx);
            }
        }
    }

    fn delete_prompt_with_confirm(
        &mut self,
        app: AppKind,
        prompt: &domain::Prompt,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if prompt.enabled {
            notify_error(t!("prompts.cannot_delete_enabled").to_string(), window, cx);
            return;
        }
        let view = cx.entity().downgrade();
        let prompt_id = prompt.id.clone();
        let prompt_name = prompt.name.clone();

        window.open_dialog(cx, move |dialog, _window, cx| {
            let view = view.clone();
            let prompt_id = prompt_id.clone();
            let prompt_name = prompt_name.clone();

            dialog
                .confirm()
                .title(t!("prompts.confirm_delete_title").to_string())
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(cx.theme().muted_foreground)
                        .child(
                            t!(
                                "prompts.confirm_delete_message",
                                name = prompt_name.as_str()
                            )
                            .to_string(),
                        ),
                )
                .on_ok(move |_, window, cx| {
                    let prompt_id = prompt_id.clone();
                    if let Some(app_view) = view.upgrade() {
                        app_view.update(cx, |this, cx| {
                            if let Err(e) = this.workspace.delete_prompt(app, &prompt_id) {
                                notify_error(format!("删除失败: {e}"), window, cx);
                            } else {
                                notify_success("提示词已删除", window, cx);
                                this.refresh_prompts(cx);
                            }
                        });
                    }
                    true
                })
        });
    }

    fn import_prompt_from_live(
        &mut self,
        app: AppKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self.workspace.import_prompt_from_file(app) {
            Ok(p) => {
                notify_success(
                    format!("{}: {}", t!("prompts.import_success"), p.name),
                    window,
                    cx,
                );
                self.refresh_prompts(cx);
            }
            Err(e) => {
                notify_error(format!("从文件导入失败: {e}"), window, cx);
            }
        }
    }

    fn open_create_prompt(&mut self, app: AppKind, window: &mut Window, cx: &mut Context<Self>) {
        let view = cx.entity().downgrade();
        let name_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t!("prompts.name_placeholder").to_string())
        });
        let desc_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("prompts.description_placeholder").to_string())
        });
        let placeholder_content = format!(
            "# {}\n\n{}",
            domain::prompt_filename(app),
            t!("prompts.content_placeholder")
        );
        let content_input = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor("markdown")
                .placeholder(placeholder_content)
        });

        window.open_dialog(cx, move |dialog, _window, cx| {
            let view = view.clone();
            let name_input = name_input.clone();
            let desc_input = desc_input.clone();
            let content_input = content_input.clone();
            let border_color = cx.theme().border;

            dialog
                .width(px(680.))
                .title(t!("prompts.add").to_string())
                .child(
                    v_flex()
                        .w_full()
                        .gap(px(14.))
                        .child(
                            v_flex()
                                .gap(px(4.))
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(t!("prompts.name").to_string()),
                                )
                                .child(Input::new(&name_input).cleanable(true)),
                        )
                        .child(
                            v_flex()
                                .gap(px(4.))
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(t!("prompts.description").to_string()),
                                )
                                .child(Input::new(&desc_input).cleanable(true)),
                        )
                        .child(
                            v_flex()
                                .gap(px(4.))
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(t!("prompts.content").to_string()),
                                )
                                .child(
                                    div()
                                        .h(px(240.))
                                        .w_full()
                                        .rounded(px(8.))
                                        .border_1()
                                        .border_color(border_color)
                                        .p(px(8.))
                                        .overflow_y_scrollbar()
                                        .child(Input::new(&content_input)),
                                ),
                        ),
                )
                .footer(move |_ok, _cancel, _window, _cx| {
                    let view = view.clone();
                    let name_input = name_input.clone();
                    let desc_input = desc_input.clone();
                    let content_input = content_input.clone();

                    vec![h_flex()
                        .w_full()
                        .justify_end()
                        .gap(px(8.))
                        .child(
                            Button::new("cancel-prompt")
                                .label(t!("common.cancel").to_string())
                                .on_click(|_, window, cx| {
                                    window.close_dialog(cx);
                                }),
                        )
                        .child(
                            Button::new("save-prompt")
                                .primary()
                                .label(t!("common.save").to_string())
                                .on_click(move |_, window, cx| {
                                    let name = name_input.read(cx).value().trim().to_string();
                                    if name.is_empty() {
                                        notify_error("请输入提示词名称", window, cx);
                                        return;
                                    }
                                    let description = {
                                        let d = desc_input.read(cx).value().trim().to_string();
                                        if d.is_empty() {
                                            None
                                        } else {
                                            Some(d)
                                        }
                                    };
                                    let content = content_input.read(cx).value().to_string();
                                    let now = std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_secs()
                                        as i64;
                                    let id = format!("prompt-{now}");
                                    let prompt = domain::Prompt {
                                        id: id.clone(),
                                        name,
                                        content,
                                        description,
                                        enabled: false,
                                        created_at: Some(now),
                                        updated_at: Some(now),
                                    };
                                    if let Some(app_view) = view.upgrade() {
                                        app_view.update(cx, |this, cx| {
                                            if let Err(e) =
                                                this.workspace.save_prompt(app, &id, prompt)
                                            {
                                                notify_error(format!("保存失败: {e}"), window, cx);
                                            } else {
                                                notify_success("提示词已保存", window, cx);
                                                this.refresh_prompts(cx);
                                                window.close_dialog(cx);
                                            }
                                        });
                                    }
                                }),
                        )
                        .into_any_element()]
                })
        });
    }

    fn open_edit_prompt(
        &mut self,
        app: AppKind,
        prompt: &domain::Prompt,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let view = cx.entity().downgrade();
        let prompt_id = prompt.id.clone();
        let is_enabled = prompt.enabled;
        let created_at = prompt.created_at;

        let name_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("prompts.name_placeholder").to_string())
                .default_value(prompt.name.clone())
        });
        let desc_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("prompts.description_placeholder").to_string())
                .default_value(prompt.description.clone().unwrap_or_default())
        });
        let content_input = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor("markdown")
                .default_value(prompt.content.clone())
        });

        window.open_dialog(cx, move |dialog, _window, cx| {
            let view = view.clone();
            let name_input = name_input.clone();
            let desc_input = desc_input.clone();
            let content_input = content_input.clone();
            let prompt_id = prompt_id.clone();
            let border_color = cx.theme().border;

            dialog
                .width(px(680.))
                .title(t!("prompts.edit").to_string())
                .child(
                    v_flex()
                        .w_full()
                        .gap(px(14.))
                        .child(
                            v_flex()
                                .gap(px(4.))
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(t!("prompts.name").to_string()),
                                )
                                .child(Input::new(&name_input).cleanable(true)),
                        )
                        .child(
                            v_flex()
                                .gap(px(4.))
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(t!("prompts.description").to_string()),
                                )
                                .child(Input::new(&desc_input).cleanable(true)),
                        )
                        .child(
                            v_flex()
                                .gap(px(4.))
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(t!("prompts.content").to_string()),
                                )
                                .child(
                                    div()
                                        .h(px(240.))
                                        .w_full()
                                        .rounded(px(8.))
                                        .border_1()
                                        .border_color(border_color)
                                        .p(px(8.))
                                        .overflow_y_scrollbar()
                                        .child(Input::new(&content_input)),
                                ),
                        ),
                )
                .footer(move |_ok, _cancel, _window, _cx| {
                    let view = view.clone();
                    let name_input = name_input.clone();
                    let desc_input = desc_input.clone();
                    let content_input = content_input.clone();
                    let prompt_id = prompt_id.clone();

                    vec![h_flex()
                        .w_full()
                        .justify_end()
                        .gap(px(8.))
                        .child(
                            Button::new("cancel-edit-prompt")
                                .label(t!("common.cancel").to_string())
                                .on_click(|_, window, cx| {
                                    window.close_dialog(cx);
                                }),
                        )
                        .child(
                            Button::new("save-edit-prompt")
                                .primary()
                                .label(t!("common.save").to_string())
                                .on_click(move |_, window, cx| {
                                    let name = name_input.read(cx).value().trim().to_string();
                                    if name.is_empty() {
                                        notify_error("请输入提示词名称", window, cx);
                                        return;
                                    }
                                    let description = {
                                        let d = desc_input.read(cx).value().trim().to_string();
                                        if d.is_empty() {
                                            None
                                        } else {
                                            Some(d)
                                        }
                                    };
                                    let content = content_input.read(cx).value().to_string();
                                    let now = std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_secs()
                                        as i64;
                                    let updated = domain::Prompt {
                                        id: prompt_id.clone(),
                                        name,
                                        content,
                                        description,
                                        enabled: is_enabled,
                                        created_at,
                                        updated_at: Some(now),
                                    };
                                    if let Some(app_view) = view.upgrade() {
                                        app_view.update(cx, |this, cx| {
                                            if let Err(e) =
                                                this.workspace.save_prompt(app, &prompt_id, updated)
                                            {
                                                notify_error(format!("保存失败: {e}"), window, cx);
                                            } else {
                                                notify_success("提示词已保存", window, cx);
                                                this.refresh_prompts(cx);
                                                window.close_dialog(cx);
                                            }
                                        });
                                    }
                                }),
                        )
                        .into_any_element()]
                })
        });
    }

    fn render_prompts_page(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let dark = theme.is_dark();
        let app = self.prompts_app;
        let search_value = self.prompts_search.read(cx).value().to_lowercase();
        let search = search_value.trim().to_lowercase();

        let visible_prompts: Vec<domain::Prompt> = self
            .prompts_list
            .iter()
            .filter(|p| {
                if search.is_empty() {
                    true
                } else {
                    p.name.to_lowercase().contains(&search)
                        || p.description
                            .as_deref()
                            .unwrap_or_default()
                            .to_lowercase()
                            .contains(&search)
                        || p.content.to_lowercase().contains(&search)
                }
            })
            .cloned()
            .collect();

        let active_prompt = self.prompts_list.iter().find(|p| p.enabled);
        let target_file_str = domain::prompt_display_path(app);

        let apps = [
            (AppKind::Codex, CustomIcon::OpenAI, "Codex"),
            (AppKind::Claude, CustomIcon::Claude, "Claude Code"),
            (AppKind::Grok, CustomIcon::Grok, "Grok Build"),
            (AppKind::OpenCode, CustomIcon::OpenCode, "OpenCode"),
            (AppKind::Pi, CustomIcon::Pi, "Pi"),
            (AppKind::Cursor, CustomIcon::Cursor, "Cursor"),
            (AppKind::ZCode, CustomIcon::ZCode, "ZCode"),
            (AppKind::WorkBuddy, CustomIcon::WorkBuddy, "WorkBuddy"),
        ];

        let app_chips = apps
            .into_iter()
            .map(|(kind, icon, label)| {
                let selected = self.prompts_app == kind;
                Button::new(SharedString::from(format!("prompt-app-{}", kind.as_str())))
                    .outline()
                    .small()
                    .selected(selected)
                    .icon(icon)
                    .label(label)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.switch_prompts_app(kind, cx);
                    }))
            })
            .collect::<Vec<_>>();

        v_flex()
            .w_full()
            .p(px(24.))
            .gap(px(16.))
            // Header
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .gap(px(12.))
                    .child(
                        h_flex()
                            .items_center()
                            .gap(px(12.))
                            .child(
                                Button::new("prompts-back")
                                    .ghost()
                                    .icon(IconName::ArrowLeft)
                                    .on_click(cx.listener(|this, _, _window, cx| {
                                        this.set_route(Route::Dashboard, cx);
                                    })),
                            )
                            .child(
                                div()
                                    .text_size(px(18.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(theme.foreground)
                                    .child(
                                        t!("prompts.title", app = app.display_name()).to_string(),
                                    ),
                            ),
                    )
                    .child(
                        h_flex()
                            .items_center()
                            .gap(px(8.))
                            .child(
                                Button::new("import-prompt-btn")
                                    .outline()
                                    .icon(IconName::ArrowDown)
                                    .label(t!("prompts.import_from_file").to_string())
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.import_prompt_from_live(app, window, cx);
                                    })),
                            )
                            .child(
                                Button::new("add-prompt-btn")
                                    .primary()
                                    .icon(IconName::Plus)
                                    .label(t!("prompts.add").to_string())
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.open_create_prompt(app, window, cx);
                                    })),
                            ),
                    ),
            )
            // App switcher tabs
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .gap(px(6.))
                    .children(app_chips),
            )
            // Search & Info Bar
            .child(
                v_flex()
                    .w_full()
                    .gap(px(10.))
                    .child(
                        h_flex().w_full().items_center().child(
                            div()
                                .flex_1()
                                .child(Input::new(&self.prompts_search).cleanable(true)),
                        ),
                    )
                    .child(
                        theme::tile(cx).w_full().p(px(12.)).child(
                            h_flex()
                                .w_full()
                                .items_center()
                                .justify_between()
                                .gap(px(12.))
                                .child(
                                    h_flex()
                                        .items_center()
                                        .gap(px(8.))
                                        .child(
                                            div()
                                                .text_size(px(12.5))
                                                .text_color(theme.muted_foreground)
                                                .child(
                                                    t!(
                                                        "prompts.count",
                                                        count = self.prompts_list.len()
                                                    )
                                                    .to_string(),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(12.5))
                                                .text_color(theme.muted_foreground)
                                                .child("·"),
                                        )
                                        .child(if let Some(active) = active_prompt {
                                            Tag::success().small().child(
                                                t!(
                                                    "prompts.enabled_name",
                                                    name = active.name.as_str()
                                                )
                                                .to_string(),
                                            )
                                        } else {
                                            Tag::secondary()
                                                .small()
                                                .child(t!("prompts.none_enabled").to_string())
                                        }),
                                )
                                .child(
                                    h_flex()
                                        .items_center()
                                        .gap(px(6.))
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .text_color(theme.muted_foreground)
                                                .child("目标文件:"),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .font_weight(FontWeight::MEDIUM)
                                                .font_family(".AppleSystemUIFontMonospaced")
                                                .text_color(if dark {
                                                    rgb(0x60A5FA)
                                                } else {
                                                    rgb(0x2563EB)
                                                })
                                                .child(target_file_str),
                                        ),
                                ),
                        ),
                    ),
            )
            // Prompts List
            .child(if visible_prompts.is_empty() {
                if !search.is_empty() {
                    empty_search_state(cx).into_any_element()
                } else {
                    v_flex()
                        .w_full()
                        .py(px(40.))
                        .items_center()
                        .justify_center()
                        .gap(px(12.))
                        .child(
                            div()
                                .size(px(48.))
                                .rounded_full()
                                .bg(theme.secondary)
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    Icon::new(CustomIcon::BookOpen)
                                        .size(px(24.))
                                        .text_color(theme.muted_foreground),
                                ),
                        )
                        .child(
                            div()
                                .text_size(px(15.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(theme.foreground)
                                .child(t!("prompts.empty").to_string()),
                        )
                        .child(
                            div()
                                .text_size(px(12.5))
                                .text_color(theme.muted_foreground)
                                .child(t!("prompts.empty_description").to_string()),
                        )
                        .child(
                            h_flex()
                                .gap(px(8.))
                                .pt(px(8.))
                                .child(
                                    Button::new("empty-import-prompt-btn")
                                        .outline()
                                        .small()
                                        .icon(IconName::ArrowDown)
                                        .label(t!("prompts.import_from_file").to_string())
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.import_prompt_from_live(app, window, cx);
                                        })),
                                )
                                .child(
                                    Button::new("empty-add-prompt-btn")
                                        .primary()
                                        .small()
                                        .icon(IconName::Plus)
                                        .label(t!("prompts.add").to_string())
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.open_create_prompt(app, window, cx);
                                        })),
                                ),
                        )
                        .into_any_element()
                }
            } else {
                v_flex()
                    .w_full()
                    .gap(px(12.))
                    .children(
                        visible_prompts
                            .iter()
                            .map(|prompt| self.render_prompt_card(prompt, cx)),
                    )
                    .into_any_element()
            })
    }

    fn render_prompt_card(
        &self,
        prompt: &domain::Prompt,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme().clone();
        let dark = theme.is_dark();
        let app = self.prompts_app;
        let prompt_clone = prompt.clone();
        let id = prompt.id.clone();
        let is_enabled = prompt.enabled;

        let preview_lines = prompt
            .content
            .lines()
            .take(4)
            .collect::<Vec<_>>()
            .join("\n");
        let preview_text = if preview_lines.trim().is_empty() {
            "(无内容)".to_string()
        } else {
            preview_lines
        };

        let card = theme::tile(cx).w_full().p(px(14.)).gap(px(10.));
        let card = if is_enabled {
            card.border_1()
                .border_color(if dark { rgb(0x10B981) } else { rgb(0x059669) })
                .bg(if dark {
                    rgba(0x064E3B26)
                } else {
                    rgba(0xECFDF5FA)
                })
                .shadow_sm()
        } else {
            card
        };

        card.child(
            h_flex()
                .w_full()
                .items_start()
                .justify_between()
                .gap(px(12.))
                // Left: Switch + Name + Desc
                .child(
                    h_flex()
                        .flex_1()
                        .items_start()
                        .gap(px(10.))
                        .child(
                            div()
                                .id(SharedString::from(format!("switch-prompt-{}", prompt.id)))
                                .cursor_pointer()
                                .pt(px(2.))
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.toggle_prompt_enabled(app, &id, is_enabled, window, cx);
                                }))
                                .child(self.render_switch(prompt.enabled, cx)),
                        )
                        .child(
                            v_flex()
                                .flex_1()
                                .min_w_0()
                                .gap(px(4.))
                                .child(
                                    h_flex()
                                        .items_center()
                                        .gap(px(8.))
                                        .child(
                                            div()
                                                .text_size(px(14.5))
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(theme.foreground)
                                                .child(prompt.name.clone()),
                                        )
                                        .when(is_enabled, |this| {
                                            this.child(Tag::success().small().child("启用中"))
                                        }),
                                )
                                .when_some(prompt.description.as_ref(), |this, desc| {
                                    this.child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(theme.muted_foreground)
                                            .child(desc.clone()),
                                    )
                                }),
                        ),
                )
                // Right: Action buttons
                .child(
                    h_flex()
                        .items_center()
                        .gap(px(6.))
                        .child(
                            Button::new(SharedString::from(format!("edit-prompt-{}", prompt.id)))
                                .ghost()
                                .xsmall()
                                .icon(IconName::Settings2)
                                .label(t!("prompts.edit").to_string())
                                .on_click(cx.listener({
                                    let prompt_clone = prompt_clone.clone();
                                    move |this, _, window, cx| {
                                        this.open_edit_prompt(app, &prompt_clone, window, cx);
                                    }
                                })),
                        )
                        .child(
                            Button::new(SharedString::from(format!("delete-prompt-{}", prompt.id)))
                                .ghost()
                                .xsmall()
                                .icon(IconName::Delete)
                                .label(t!("prompts.delete").to_string())
                                .disabled(prompt.enabled)
                                .on_click(cx.listener({
                                    let prompt_clone = prompt_clone.clone();
                                    move |this, _, window, cx| {
                                        this.delete_prompt_with_confirm(
                                            app,
                                            &prompt_clone,
                                            window,
                                            cx,
                                        );
                                    }
                                })),
                        ),
                ),
        )
        // Markdown preview snippet box
        .child(
            div()
                .w_full()
                .p(px(8.))
                .rounded(px(6.))
                .bg(theme.secondary.opacity(0.6))
                .border_1()
                .border_color(theme.border.opacity(0.5))
                .text_size(px(12.))
                .line_height(px(18.))
                .font_family(".AppleSystemUIFontMonospaced")
                .text_color(theme.muted_foreground)
                .child(preview_text),
        )
    }

    fn render_usage_script_page(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let provider = self
            .usage_script_provider
            .as_deref()
            .and_then(|id| self.workspace.provider(id).ok().flatten());
        let provider_name = provider
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "未知服务商".into());
        let is_new_api = self.usage_template == domain::TEMPLATE_NEW_API;

        let template_chip =
            |id: &'static str, name: String, template: &'static str, cx: &mut Context<Self>| {
                let selected = self.usage_template == template;
                Button::new(id)
                    .outline()
                    .xsmall()
                    .selected(selected)
                    .label(name)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.set_usage_template(template.to_string(), window, cx);
                    }))
            };

        let result_summary = self.usage_last_result.as_ref().map(|result| {
            let text = if result.success {
                usage_summary_text(result)
            } else {
                result
                    .error
                    .clone()
                    .unwrap_or_else(|| "查询失败".to_string())
            };
            let tag = if result.success {
                Tag::success().small().child(format!("上次查询: {text}"))
            } else {
                Tag::danger().small().child(format!("上次查询失败: {text}"))
            };
            div().child(tag)
        });

        v_flex()
            .w_full()
            .p(px(24.))
            .gap(px(16.))
            // Header
            .child(
                h_flex()
                    .items_center()
                    .gap(px(12.))
                    .child(
                        Button::new("usage-script-back")
                            .ghost()
                            .icon(IconName::ArrowLeft)
                            .on_click(cx.listener(|this, _, _window, cx| {
                                let back = if this.previous_route == Route::UsageScript {
                                    Route::Dashboard
                                } else {
                                    this.previous_route
                                };
                                this.set_route(back, cx);
                            })),
                    )
                    .child(
                        div()
                            .text_size(px(18.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.foreground)
                            .child(format!("{} - {}", t!("usage_script.title"), provider_name)),
                    ),
            )
            .children(result_summary)
            // Enable toggle
            .child(
                theme::tile(cx).child(
                    h_flex()
                        .w_full()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_size(px(14.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(theme.foreground)
                                .child(t!("usage_script.enable").to_string()),
                        )
                        .child(
                            div()
                                .id("usage-script-enable")
                                .cursor_pointer()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.usage_enabled = !this.usage_enabled;
                                    cx.notify();
                                }))
                                .child(self.render_switch(self.usage_enabled, cx)),
                        ),
                ),
            )
            // Credentials & timing
            .child(
                theme::tile(cx).child(
                    v_flex()
                        .w_full()
                        .gap(px(12.))
                        .child(
                            div()
                                .text_size(px(14.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(theme.foreground)
                                .child(t!("usage_script.template").to_string()),
                        )
                        .child(
                            h_flex()
                                .gap(px(6.))
                                .child(template_chip(
                                    "usage-template-custom",
                                    "自定义".to_string(),
                                    domain::TEMPLATE_CUSTOM,
                                    cx,
                                ))
                                .child(template_chip(
                                    "usage-template-general",
                                    t!("usage_script.template_general").to_string(),
                                    domain::TEMPLATE_GENERAL,
                                    cx,
                                ))
                                .child(template_chip(
                                    "usage-template-newapi",
                                    "NewAPI".to_string(),
                                    domain::TEMPLATE_NEW_API,
                                    cx,
                                )),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(theme.muted_foreground)
                                .child(t!("usage_script.credentials_hint").to_string()),
                        )
                        .child(
                            h_flex()
                                .w_full()
                                .gap(px(12.))
                                .child(
                                    v_flex()
                                        .flex_1()
                                        .gap(px(4.))
                                        .child(field_label(
                                            t!("usage_script.api_key").to_string(),
                                            cx,
                                        ))
                                        .child(Input::new(&self.usage_api_key)),
                                )
                                .child(
                                    v_flex()
                                        .flex_1()
                                        .gap(px(4.))
                                        .child(field_label(
                                            t!("usage_script.base_url").to_string(),
                                            cx,
                                        ))
                                        .child(Input::new(&self.usage_base_url)),
                                ),
                        )
                        .when(is_new_api, |this| {
                            this.child(
                                h_flex()
                                    .w_full()
                                    .gap(px(12.))
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .gap(px(4.))
                                            .child(field_label(
                                                t!("usage_script.access_token").to_string(),
                                                cx,
                                            ))
                                            .child(Input::new(&self.usage_access_token)),
                                    )
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .gap(px(4.))
                                            .child(field_label(
                                                t!("usage_script.user_id").to_string(),
                                                cx,
                                            ))
                                            .child(Input::new(&self.usage_user_id)),
                                    ),
                            )
                        })
                        .child(
                            h_flex()
                                .w_full()
                                .gap(px(12.))
                                .child(
                                    v_flex()
                                        .flex_1()
                                        .gap(px(4.))
                                        .child(field_label(
                                            t!("usage_script.timeout").to_string(),
                                            cx,
                                        ))
                                        .child(Input::new(&self.usage_timeout)),
                                )
                                .child(
                                    v_flex()
                                        .flex_1()
                                        .gap(px(4.))
                                        .child(field_label(
                                            t!("usage_script.interval").to_string(),
                                            cx,
                                        ))
                                        .child(Input::new(&self.usage_interval)),
                                ),
                        ),
                ),
            )
            // Extractor code
            .child(
                theme::tile(cx).child(
                    v_flex()
                        .w_full()
                        .gap(px(8.))
                        .child(
                            h_flex()
                                .w_full()
                                .items_center()
                                .justify_between()
                                .child(
                                    div()
                                        .text_size(px(14.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(theme.foreground)
                                        .child(t!("usage_script.code").to_string()),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .text_color(theme.muted_foreground)
                                        .child(t!("usage_script.code_hint").to_string()),
                                ),
                        )
                        .child(
                            div()
                                .h(px(420.))
                                .font_family("Menlo")
                                .border_1()
                                .border_color(theme.border)
                                .rounded(px(8.))
                                .overflow_hidden()
                                .child(Input::new(&self.usage_code).h_full()),
                        ),
                ),
            )
            // Script guide
            .child(
                theme::tile(cx).child(
                    v_flex()
                        .w_full()
                        .gap(px(8.))
                        .child(
                            div()
                                .text_size(px(14.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(theme.foreground)
                                .child(t!("usage_script.guide_title").to_string()),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(theme.foreground)
                                .child(t!("usage_script.guide_format").to_string()),
                        )
                        .child(
                            div()
                                .p(px(12.))
                                .rounded(px(8.))
                                .bg(theme.secondary.opacity(0.5))
                                .font_family("Menlo")
                                .text_size(px(11.5))
                                .text_color(theme.foreground)
                                .child(t!("usage_script.guide_sample").to_string()),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(theme.foreground)
                                .child(t!("usage_script.guide_fields_title").to_string()),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(theme.muted_foreground)
                                .child(t!("usage_script.guide_fields").to_string()),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(theme.foreground)
                                .child(t!("usage_script.guide_tips_title").to_string()),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(theme.muted_foreground)
                                .child(t!("usage_script.guide_tips").to_string()),
                        ),
                ),
            )
            // Actions
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_end()
                    .gap(px(8.))
                    .child(
                        Button::new("usage-script-cancel")
                            .outline()
                            .label(t!("usage_script.cancel").to_string())
                            .on_click(cx.listener(|this, _, _window, cx| {
                                let back = if this.previous_route == Route::UsageScript {
                                    Route::Dashboard
                                } else {
                                    this.previous_route
                                };
                                this.set_route(back, cx);
                            })),
                    )
                    .child(
                        Button::new("usage-script-test")
                            .outline()
                            .icon(CustomIcon::Activity)
                            .label(if self.usage_querying {
                                t!("usage_script.testing").to_string()
                            } else {
                                t!("usage_script.test").to_string()
                            })
                            .disabled(self.usage_querying)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.test_usage_script(window, cx);
                            })),
                    )
                    .child(
                        Button::new("usage-script-save")
                            .primary()
                            .icon(IconName::Check)
                            .label(t!("usage_script.save").to_string())
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.save_usage_script(window, cx);
                            })),
                    ),
            )
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
                        .when_some(
                            self.usage_badges.get(&provider.id).cloned(),
                            |this, (result, fetched_at)| {
                                let muted = cx.theme().muted_foreground;
                                let refresh_id = provider.id.clone();
                                let refreshing = self.usage_refreshing.contains(&provider.id);
                                let now_secs = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .map(|d| d.as_secs() as i64)
                                    .unwrap_or(0);
                                this.child(
                                    v_flex()
                                        .items_end()
                                        .gap(px(2.))
                                        .child(
                                            h_flex()
                                                .items_center()
                                                .gap(px(2.))
                                                .child(
                                                    div()
                                                        .text_size(px(11.))
                                                        .text_color(muted)
                                                        .child(relative_time_text(
                                                            now_secs - fetched_at,
                                                            self.language,
                                                        )),
                                                )
                                                .child(
                                                    Button::new(SharedString::from(format!(
                                                        "usage-refresh-{}",
                                                        provider.id
                                                    )))
                                                    .ghost()
                                                    .xsmall()
                                                    .icon(CustomIcon::RotateCw)
                                                    .disabled(refreshing)
                                                    .on_click(cx.listener(
                                                        move |this, _, window, cx| {
                                                            this.start_usage_refresh(
                                                                &refresh_id,
                                                                window,
                                                                cx,
                                                            );
                                                        },
                                                    )),
                                                ),
                                        )
                                        .child(usage_value_line(&result, muted)),
                                )
                            },
                        )
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
                            Button::new(SharedString::from(format!("usage-{}", provider.id)))
                                .outline()
                                .small()
                                .icon(IconName::ChartPie)
                                .label(t!("provider.usage_query").to_string())
                                .tooltip("配置用量查询(余额 / 套餐)")
                                .on_click(cx.listener({
                                    let id = id.clone();
                                    move |this, _, window, cx| {
                                        this.open_usage_script(&id, window, cx)
                                    }
                                })),
                        )
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

        let auth_matches = query.is_empty()
            || "认证 auth oauth 登录 login chatgpt codex openai xai grok 账号 account"
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
                    .when(auth_matches, |this| {
                        this.child(self.render_settings_sidebar_item(
                            SettingsTab::Auth,
                            IconName::CircleUser,
                            t!("settings.auth").to_string(),
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
                        !general_matches && !auth_matches && !advanced_matches && !about_matches,
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
                if tab == SettingsTab::Auth {
                    this.reload_oauth_statuses();
                }
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
                                    // .child(self.render_app_toggle_chip("amp", "Amp", CustomIcon::Amp, rgb(0xEA580C).into(), cx))
                                    .child(self.render_app_toggle_chip("claude", "Claude Code", CustomIcon::Claude, rgb(0xD97757).into(), cx))
                                    // .child(self.render_app_toggle_chip("claude-desktop", "Claude Desktop", CustomIcon::Claude, rgb(0xD97757).into(), cx))
                                    .child(self.render_app_toggle_chip("codex", "Codex", CustomIcon::OpenAI, rgb(0x10A37F).into(), cx))
                                    .child(self.render_app_toggle_chip("cursor", "Cursor", CustomIcon::Cursor, rgb(0x6366F1).into(), cx))
                                    // .child(self.render_app_toggle_chip("deepseek", "DeepSeek Harness", CustomIcon::DeepSeek, rgb(0x3B82F6).into(), cx))
                                    // .child(self.render_app_toggle_chip("fx", "Fx", CustomIcon::Fx, rgb(0x4B5563).into(), cx))
                                    .child(self.render_app_toggle_chip("opencode", "OpenCode", CustomIcon::OpenCode, rgb(0x0284C7).into(), cx))
                                    .child(self.render_app_toggle_chip("grok", "Grok Build", CustomIcon::Grok, rgb(0x8B5CF6).into(), cx))
                                    // .child(self.render_app_toggle_chip("kimi", "Kimi Code", CustomIcon::Kimi, rgb(0x2563EB).into(), cx))
                                    // .child(self.render_app_toggle_chip("ohmypi", "Oh My Pi", CustomIcon::OhMyPi, rgb(0xEC4899).into(), cx))
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
                                                            .child(t!("app.name").to_string()),
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
            SettingsTab::Auth => self.render_auth_settings(cx).into_any_element(),
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

    fn render_codex_auth_selector(
        &self,
        form: &FormDraft,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme().clone();
        let auth_status = self.codex_oauth_status.as_ref();
        let is_authenticated = auth_status.map(|s| s.authenticated).unwrap_or(false);
        let account_name = auth_status
            .and_then(|s| s.account.clone())
            .unwrap_or_else(|| "ChatGPT OAuth 账号".to_string());

        let (current_label, current_sublabel) =
            if form.codex_auth_mode == "oauth" && is_authenticated {
                (account_name.clone(), "ChatGPT OAuth 账号 (已认证)")
            } else {
                (
                    "跟随 Codex 登录".to_string(),
                    "账号会随 Codex CLI 当前登录变化",
                )
            };

        let is_open = form.codex_auth_dropdown_open;

        v_flex()
            .w_full()
            .gap(px(6.))
            .child(
                div()
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme.foreground)
                    .child("登录方式"),
            )
            .child(
                v_flex()
                    .w_full()
                    .gap(px(4.))
                    .child(
                        // Dropdown Trigger Button
                        h_flex()
                            .id("codex-auth-trigger-btn")
                            .w_full()
                            .p(px(10.))
                            .rounded(px(6.))
                            .border_1()
                            .border_color(theme.border)
                            .bg(theme.secondary.opacity(0.35))
                            .hover(|s| s.bg(theme.secondary.opacity(0.6)))
                            .cursor_pointer()
                            .items_center()
                            .justify_between()
                            .on_click(cx.listener(|this, _, _, cx| {
                                if let Some(form) = this.form.as_mut() {
                                    form.codex_auth_dropdown_open = !form.codex_auth_dropdown_open;
                                    cx.notify();
                                }
                            }))
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap(px(10.))
                                    .child(
                                        div()
                                            .size(px(28.))
                                            .rounded(px(6.))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .bg(theme.secondary.opacity(0.6))
                                            .child(
                                                Icon::new(CustomIcon::OpenAI)
                                                    .size(px(16.))
                                                    .text_color(rgb(0x10A37F)),
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
                                                    .child(current_label),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(11.))
                                                    .text_color(theme.muted_foreground)
                                                    .child(current_sublabel),
                                            ),
                                    ),
                            )
                            .child(
                                Icon::new(if is_open {
                                    IconName::ChevronUp
                                } else {
                                    IconName::ChevronDown
                                })
                                .size(px(16.))
                                .text_color(theme.muted_foreground),
                            ),
                    )
                    .when(is_open, |this| {
                        let is_auth = is_authenticated;
                        let acc = account_name.clone();
                        let current_mode = form.codex_auth_mode.clone();
                        let mut follow_item = h_flex()
                            .id("codex-auth-follow-item")
                            .w_full()
                            .p(px(8.))
                            .rounded(px(4.))
                            .items_center()
                            .justify_between()
                            .hover(|s| s.bg(theme.secondary.opacity(0.4)))
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| {
                                if let Some(form) = this.form.as_mut() {
                                    form.codex_auth_mode = "follow".to_string();
                                    form.codex_auth_dropdown_open = false;
                                    cx.notify();
                                }
                            }))
                            .child(
                                v_flex()
                                    .gap(px(2.))
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(theme.foreground)
                                            .child("跟随 Codex 登录"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.))
                                            .text_color(theme.muted_foreground)
                                            .child("账号会随 Codex CLI 当前登录变化"),
                                    ),
                            );
                        if current_mode != "oauth" {
                            follow_item = follow_item.child(
                                Icon::new(IconName::Check)
                                    .size(px(14.))
                                    .text_color(theme.primary),
                            );
                        }

                        this.child(
                            v_flex()
                                .w_full()
                                .p(px(4.))
                                .rounded(px(6.))
                                .border_1()
                                .border_color(theme.border)
                                .bg(theme.background)
                                .gap(px(2.))
                                .shadow_sm()
                                .child(
                                    // Top Action: + 添加或管理 ChatGPT 账号...
                                    h_flex()
                                        .id("codex-auth-add-btn")
                                        .w_full()
                                        .p(px(8.))
                                        .rounded(px(4.))
                                        .items_center()
                                        .gap(px(8.))
                                        .hover(|s| s.bg(theme.primary.opacity(0.12)))
                                        .cursor_pointer()
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            if let Some(form) = this.form.as_mut() {
                                                form.codex_auth_dropdown_open = false;
                                            }
                                            this.start_oauth_login(session::CODEX_PROVIDER, window, cx);
                                        }))
                                        .child(
                                            Icon::new(IconName::Plus)
                                                .size(px(14.))
                                                .text_color(theme.primary),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(theme.primary)
                                                .child("+ 添加或管理 ChatGPT 账号..."),
                                        ),
                                )
                                .child(
                                    div()
                                        .w_full()
                                        .h(px(1.))
                                        .bg(theme.border.opacity(0.5))
                                        .my(px(2.)),
                                )
                                .child(follow_item)
                                .when(is_auth, |this| {
                                    let mut oauth_item = h_flex()
                                        .id("codex-auth-oauth-item")
                                        .w_full()
                                        .p(px(8.))
                                        .rounded(px(4.))
                                        .items_center()
                                        .justify_between()
                                        .hover(|s| s.bg(theme.secondary.opacity(0.4)))
                                        .cursor_pointer()
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            if let Some(form) = this.form.as_mut() {
                                                form.codex_auth_mode = "oauth".to_string();
                                                form.codex_auth_dropdown_open = false;
                                                cx.notify();
                                            }
                                        }))
                                        .child(
                                            v_flex()
                                                .gap(px(2.))
                                                .child(
                                                    div()
                                                        .text_size(px(12.))
                                                        .font_weight(FontWeight::MEDIUM)
                                                        .text_color(theme.foreground)
                                                        .child(acc),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(11.))
                                                        .text_color(theme.muted_foreground)
                                                        .child("ChatGPT OAuth 账号 (已认证)"),
                                                ),
                                        );
                                    if current_mode == "oauth" {
                                        oauth_item = oauth_item.child(
                                            Icon::new(IconName::Check)
                                                .size(px(14.))
                                                .text_color(theme.primary),
                                        );
                                    }
                                    this.child(oauth_item)
                                })
                                .when(!is_auth, |this| {
                                    this.child(
                                        div()
                                            .p(px(8.))
                                            .text_size(px(11.))
                                            .text_color(theme.muted_foreground)
                                            .child("暂无已登录的 ChatGPT 账号 (点击上方「+ 添加或管理」进行登录)"),
                                    )
                                }),
                        )
                    }),
            )
    }

    fn render_form_page(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(form) = self.form.as_ref() else {
            return div().into_any_element();
        };

        let codex_auth_selector = if form.is_official && form.app == AppKind::Codex {
            Some(self.render_codex_auth_selector(form, cx).into_any_element())
        } else {
            None
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
        let theme = cx.theme().clone();

        v_flex()
            .w_full()
            .gap(px(16.))
            .child(
                // Header bar with breadcrumbs / back button and actions
                // （不设 w_full：依赖 stretch 铺满，justify_between 才能把操作按钮推到右侧）
                h_flex()
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
            .when(form.is_official, |this| {
                this.child(
                    theme::tile(cx).child(
                        v_flex()
                            .w_full()
                            .gap(px(14.))
                            .child(theme::tile_label("BASIC SETTINGS / 基础配置", cx))
                            .child(
                                h_flex()
                                    .w_full()
                                    .gap(px(12.))
                                    .child(
                                        div()
                                            .flex_1()
                                            .child(form_field("供应商名称", Input::new(&form.name))),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .child(form_field("备注 (可选)", Input::new(&form.notes))),
                                    ),
                            )
                            .child(form_field("官网链接", Input::new(&form.website_url)))
                            .when_some(codex_auth_selector, |this, sel| this.child(sel)),
                    ),
                )
            })
            .when(!form.is_official, |this| {
                this.child(
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
                                .child(
                                    v_flex()
                                        .gap(px(6.))
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(theme.foreground)
                                                .child(t!("provider.request_protocol").to_string()),
                                        )
                                        .child(Select::new(&form.protocol_select).placeholder(
                                            t!("provider.request_protocol_placeholder").to_string(),
                                        )),
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
                        .when(form.app != AppKind::ClaudeDesktop, |this| {
                            this.child(
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
                        })
                        .when(form.app == AppKind::Cursor, |this| {
                            this.when_some(form.thinking_effort_select.as_ref(), |this, select| {
                                this.child(
                                    v_flex()
                                        .gap(px(6.))
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(theme.foreground)
                                                .child(t!("provider.thinking_effort").to_string()),
                                        )
                                        .child(Select::new(select).placeholder(
                                            t!("provider.thinking_effort_placeholder").to_string(),
                                        )),
                                )
                            })
                        })
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
            .when(form.app == AppKind::ClaudeDesktop, |this| {
                // Claude Desktop 模型配置: 左侧标题+说明，右侧接入方式下拉
                // (对齐 cc-switch)
                let (mapping_desc, role_hint) = if form.desktop_mode
                    == domain::CLAUDE_DESKTOP_MODE_MAPPING
                {
                    (
                        t!("claude_desktop.model_config_mapping_hint").to_string(),
                        t!("claude_desktop.route_map_hint").to_string(),
                    )
                } else {
                    (
                        t!("claude_desktop.model_config_direct_hint").to_string(),
                        String::new(),
                    )
                };
                this.child(
                    theme::tile(cx).child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .gap(px(24.))
                            .child(
                                v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .gap(px(4.))
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(theme.foreground)
                                            .child(t!("claude_desktop.model_config_title").to_string()),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .line_height(px(18.))
                                            .text_color(theme.muted_foreground)
                                            .child(mapping_desc),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .flex_none()
                                    .items_center()
                                    .gap(px(8.))
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .text_color(theme.foreground)
                                            .child(t!("claude_desktop.model_mode_label").to_string()),
                                    )
                                    .children(form.desktop_mode_select.as_ref().map(|select| {
                                        div()
                                            .w(px(150.))
                                            .child(Select::new(select).small())
                                    })),
                            ),
                    ),
                )
                .when(
                    form.desktop_mode == domain::CLAUDE_DESKTOP_MODE_MAPPING,
                    |this| {
                        this.child(
                            theme::tile(cx).child(
                                v_flex()
                                    .w_full()
                                    .gap(px(12.))
                                    .child(
                                        // 不设 w_full：依赖 v_flex 的 align-items: stretch
                                        // 铺满整行，justify_between 才能把按钮推到最右侧
                                        h_flex()
                                            .items_center()
                                            .justify_between()
                                            .gap(px(12.))
                                            .child(
                                                div()
                                                    .text_size(px(14.))
                                                    .font_weight(FontWeight::SEMIBOLD)
                                                    .text_color(theme.foreground)
                                                    .child(t!("claude_desktop.route_map_title").to_string()),
                                            )
                                            .child(
                                                Button::new("desktop-fetch-models-btn")
                                                    .outline()
                                                    .small()
                                                    .icon(CustomIcon::Download)
                                                    .label(t!("provider.fetch_models").to_string())
                                                    .disabled(form.is_fetching_models)
                                                    .on_click(cx.listener(
                                                        |this, _, window, cx| {
                                                            this.fetch_models_for_form(
                                                                window, cx,
                                                            );
                                                        },
                                                    )),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .line_height(px(18.))
                                            .text_color(theme.muted_foreground)
                                            .child(role_hint),
                                    )
                                    .child(
                                        h_flex()
                                            .items_center()
                                            .gap(px(12.))
                                            .px(px(8.))
                                            .py(px(6.))
                                            .rounded(px(6.))
                                            .bg(theme.secondary.opacity(0.5))
                                            .child(
                                                div()
                                                    .w(px(120.))
                                                    .text_size(px(12.))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(theme.muted_foreground)
                                                    .child(t!("claude_desktop.route_role_label").to_string()),
                                            )
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .min_w(px(140.))
                                                    .text_size(px(12.))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(theme.muted_foreground)
                                                    .child(t!("claude_desktop.label_override_label").to_string()),
                                            )
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .min_w(px(220.))
                                                    .text_size(px(12.))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(theme.muted_foreground)
                                                    .child(t!("claude_desktop.upstream_model_label").to_string()),
                                            )
                                            .child(
                                                div()
                                                    .w(px(110.))
                                                    .text_size(px(12.))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(theme.muted_foreground)
                                                    .child(t!("claude_desktop.supports_1m_label").to_string()),
                                            ),
                                    )
                                    .children(
                                        form.desktop_roles
                                            .iter()
                                            .enumerate()
                                            .map(|(idx, role)| {
                                                let (role_label, _) =
                                                    domain::CLAUDE_DESKTOP_ROUTES[idx];
                                                h_flex()
                                                    .items_center()
                                                    .gap(px(12.))
                                                    .px(px(8.))
                                                    .py(px(2.))
                                                    .child(
                                                        div()
                                                            .w(px(120.))
                                                            .flex_none()
                                                            .child(
                                                                div()
                                                                    .w_full()
                                                                    .h(px(32.))
                                                                    .flex()
                                                                    .items_center()
                                                                    .px(px(12.))
                                                                    .rounded(px(6.))
                                                                    .border_1()
                                                                    .border_color(theme.border)
                                                                    .bg(theme.secondary.opacity(0.6))
                                                                    .text_size(px(13.))
                                                                    .font_weight(FontWeight::MEDIUM)
                                                                    .text_color(theme.foreground)
                                                                    .child(role_label.to_string()),
                                                            ),
                                                    )
                                                    .child(
                                                        div()
                                                            .flex_1()
                                                            .min_w(px(140.))
                                                            .child(Input::new(&role.display_name).small().cleanable(true)),
                                                    )
                                                    .child(
                                                        h_flex()
                                                            .flex_1()
                                                            .min_w(px(220.))
                                                            .items_center()
                                                            .gap(px(6.))
                                                            .child(
                                                                div()
                                                                    .flex_1()
                                                                    .child(Input::new(&role.model).small().cleanable(true)),
                                                            )
                                                            .when_some(role.model_select.as_ref(), |this, select| {
                                                                this.child(
                                                                    div()
                                                                        .w(px(130.))
                                                                        .flex_none()
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
                                                        h_flex()
                                                            .w(px(110.))
                                                            .flex_none()
                                                            .items_center()
                                                            .gap(px(8.))
                                                            .child(
                                                                Checkbox::new(
                                                                    SharedString::from(format!(
                                                                        "desktop-role-1m-{idx}"
                                                                    )),
                                                                )
                                                                .checked(role.one_m)
                                                                .on_click(cx.listener(
                                                                    move |this, _, _, cx| {
                                                                        this.toggle_desktop_role_one_m(idx, cx);
                                                                    },
                                                                )),
                                                            )
                                                            .child(
                                                                div()
                                                                    .text_size(px(13.))
                                                                    .text_color(theme.muted_foreground)
                                                                    .child("1M"),
                                                            ),
                                                    )
                                            }),
                                    ),
                            ),
                        )
                    },
                )
            })
            .when(
                form.app != AppKind::WorkBuddy && form.app != AppKind::ClaudeDesktop,
                |this| {
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
                                            v_flex()
                                                .w_full()
                                                .gap(px(4.))
                                                .child(
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
                                                ),
                                            )
                                            // Codex: 逐模型思考档位声明(多选)
                                            .when(form.app == AppKind::Codex, |this| {
                                                let effective = row
                                                    .reasoning_levels
                                                    .clone()
                                                    .unwrap_or_else(|| {
                                                        vec!["low".into(), "medium".into(), "high".into()]
                                                    });
                                                this.child(
                                                    h_flex()
                                                        .w_full()
                                                        .items_center()
                                                        .gap(px(4.))
                                                        .pl(px(4.))
                                                        .pb(px(2.))
                                                        .child(
                                                            div()
                                                                .text_size(px(11.))
                                                                .text_color(theme.muted_foreground)
                                                                .child("支持档位:"),
                                                        )
                                                        .children(domain::REASONING_LEVELS.map(
                                                            |level| {
                                                                let selected = effective
                                                                    .iter()
                                                                    .any(|l| l == level);
                                                                Button::new(SharedString::from(
                                                                    format!(
                                                                        "row-level-{idx}-{level}"
                                                                    ),
                                                                ))
                                                                .ghost()
                                                                .xsmall()
                                                                .selected(selected)
                                                                .label(level.to_string())
                                                                .on_click(cx.listener(
                                                                    move |this, _, _, cx| {
                                                                        this.toggle_catalog_row_level(
                                                                            idx, level, cx,
                                                                        );
                                                                    },
                                                                ))
                                                            },
                                                        )),
                                                )
                                            })
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
            )})})
            .child(
                // Bottom Action Buttons（不设 w_full：依赖 stretch 铺满，justify_end 才能靠右）
                h_flex()
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
                Route::Sessions => self.render_sessions_page(cx).into_any_element(),
                Route::Skills => self.render_skills_page(cx).into_any_element(),
                Route::Prompts => self.render_prompts_page(cx).into_any_element(),
                Route::Codex => self
                    .render_app_providers_page(AppKind::Codex, cx)
                    .into_any_element(),
                Route::Claude => self
                    .render_app_providers_page(AppKind::Claude, cx)
                    .into_any_element(),
                Route::ClaudeDesktop => self
                    .render_app_providers_page(AppKind::ClaudeDesktop, cx)
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
                Route::UsageScript => self.render_usage_script_page(cx).into_any_element(),
            }
        };

        let is_sessions_route = self.form.is_none() && self.route == Route::Sessions;

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
                                    .child(if is_sessions_route {
                                        div()
                                            .id("main-sessions-container")
                                            .size_full()
                                            .overflow_hidden()
                                            .child(page)
                                            .into_any_element()
                                    } else {
                                        div()
                                            .id("main-scroll")
                                            .flex_1()
                                            .min_h_0()
                                            .p(px(20.))
                                            .overflow_y_scrollbar()
                                            .child(page)
                                            .into_any_element()
                                    }),
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
                request_protocol: RequestProtocol::OpenAiResponses.as_str().into(),
                model_mappings: Vec::new(),
            }),
            AppKind::Claude | AppKind::ClaudeDesktop => ProviderForm::Claude(ClaudeForm {
                name: String::new(),
                website_url: String::new(),
                kind: ClaudeKind::ThirdParty,
                api_key: String::new(),
                base_url: String::new(),
                model: DEFAULT_CLAUDE_MODEL.to_string(),
                request_protocol: RequestProtocol::Anthropic.as_str().into(),
                model_mappings: Vec::new(),
                desktop_mode: None,
            }),
            AppKind::Grok => ProviderForm::Grok(GrokForm {
                name: String::new(),
                website_url: String::new(),
                kind: GrokKind::ThirdParty,
                api_key: String::new(),
                base_url: String::new(),
                model: DEFAULT_GROK_MODEL.to_string(),
                request_protocol: RequestProtocol::OpenAiChat.as_str().into(),
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
                request_protocol: RequestProtocol::OpenAiChat.as_str().into(),
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
                request_protocol: RequestProtocol::OpenAiChat.as_str().into(),
                model_mappings: Vec::new(),
            }),
            AppKind::Cursor => ProviderForm::Cursor(CursorForm {
                name: String::new(),
                website_url: String::new(),
                kind: CursorKind::ThirdParty,
                provider_type: domain::DEFAULT_CURSOR_PROVIDER_TYPE.to_string(),
                default_reasoning_effort: DEFAULT_THINKING_EFFORT.to_string(),
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
                request_protocol: RequestProtocol::Anthropic.as_str().into(),
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

        let (name, website_url, api_key, base_url, model, is_official, catalog_rows_data): (
            String,
            String,
            String,
            String,
            String,
            bool,
            Vec<(
                String,
                String,
                Option<u64>,
                Option<String>,
                Option<Vec<String>>,
            )>,
        ) = match &form {
            ProviderForm::Codex(f) => (
                f.name.clone(),
                f.website_url.clone(),
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
                            m.reasoning_levels.clone(),
                        )
                    })
                    .collect(),
            ),
            ProviderForm::Claude(f) => (
                f.name.clone(),
                f.website_url.clone(),
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
                            None,
                        )
                    })
                    .collect(),
            ),
            ProviderForm::Grok(f) => (
                f.name.clone(),
                f.website_url.clone(),
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
                            None,
                        )
                    })
                    .collect(),
            ),
            ProviderForm::OpenCode(f) => (
                f.name.clone(),
                f.website_url.clone(),
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
                            None,
                        )
                    })
                    .collect(),
            ),
            ProviderForm::Pi(f) => (
                f.name.clone(),
                f.website_url.clone(),
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
                            None,
                        )
                    })
                    .collect(),
            ),
            ProviderForm::Cursor(f) => (
                f.name.clone(),
                f.website_url.clone(),
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
                            None,
                        )
                    })
                    .collect(),
            ),
            ProviderForm::ZCode(f) => (
                f.name.clone(),
                f.website_url.clone(),
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
                            None,
                        )
                    })
                    .collect(),
            ),
            ProviderForm::WorkBuddy(f) => (
                f.name.clone(),
                f.website_url.clone(),
                f.api_key.clone(),
                f.base_url.clone(),
                f.model_id.clone(),
                f.kind.is_official(),
                Vec::new(),
            ),
        };

        let website_url = if website_url.trim().is_empty() && is_official {
            match app {
                AppKind::Codex => "https://chatgpt.com/codex".to_string(),
                AppKind::Claude => "https://anthropic.com".to_string(),
                AppKind::ClaudeDesktop => "https://claude.ai".to_string(),
                AppKind::Grok => "https://x.ai".to_string(),
                AppKind::OpenCode => "https://opencode.ai".to_string(),
                AppKind::Pi => "https://pi.dev".to_string(),
                AppKind::Cursor => "https://cursor.com".to_string(),
                AppKind::ZCode => "https://zcode.z.ai".to_string(),
                AppKind::WorkBuddy => "https://workbuddy.cn".to_string(),
            }
        } else {
            website_url
        };

        let protocol = match &form {
            ProviderForm::Cursor(f) => f.provider_type.clone(),
            ProviderForm::Claude(f) => f.request_protocol.clone(),
            ProviderForm::Codex(f) => f.request_protocol.clone(),
            ProviderForm::Grok(f) => f.request_protocol.clone(),
            ProviderForm::OpenCode(f) => f.request_protocol.clone(),
            ProviderForm::Pi(f) => f.request_protocol.clone(),
            ProviderForm::ZCode(f) => f.request_protocol.clone(),
            ProviderForm::WorkBuddy(_) => RequestProtocol::default_for_app(AppKind::WorkBuddy)
                .as_str()
                .to_string(),
        };
        let protocol = if protocol.trim().is_empty() {
            RequestProtocol::default_for_app(app).as_str().to_string()
        } else {
            RequestProtocol::parse(&protocol).as_str().to_string()
        };
        let thinking_effort = match &form {
            ProviderForm::Cursor(f) if !f.default_reasoning_effort.trim().is_empty() => {
                f.default_reasoning_effort.clone()
            }
            _ => DEFAULT_THINKING_EFFORT.to_string(),
        };

        let selected_index = if editing_id.is_none() {
            presets
                .iter()
                .position(|p| p.id == "custom")
                .map(|idx| gpui_component::IndexPath::default().row(idx))
        } else if name.trim().is_empty() {
            None
        } else {
            // 回显: 按端点匹配来源预设, 匹配不上(纯手填或已改过端点)则落在自定义模板
            presets
                .iter()
                .position(|p| !p.base_url.is_empty() && p.base_url.trim() == base_url.trim())
                .or_else(|| presets.iter().position(|p| p.id == "custom"))
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
            .map(|(dn, m, cw, re, rl)| {
                CatalogRowDraft::new(&dn, &m, cw, re.as_deref(), rl, &[], window, cx)
            })
            .collect();

        let (zcode_modality_text, zcode_modality_image) = match &form {
            ProviderForm::ZCode(f) => (f.modality_text, f.modality_image),
            _ => (true, true),
        };

        // Claude Desktop 默认模型映射(对齐 cc-switch)；未存储过该字段的
        // 旧数据同样按映射展示
        let desktop_mode = match &form {
            ProviderForm::Claude(f) => f.desktop_mode.clone().unwrap_or_else(|| {
                if app == AppKind::ClaudeDesktop {
                    domain::CLAUDE_DESKTOP_MODE_MAPPING.to_string()
                } else {
                    domain::CLAUDE_DESKTOP_MODE_DIRECT.to_string()
                }
            }),
            _ => domain::CLAUDE_DESKTOP_MODE_DIRECT.to_string(),
        };
        let desktop_role_sources: Vec<(String, String, bool)> = match &form {
            ProviderForm::Claude(f) => f
                .model_mappings
                .iter()
                .take(domain::CLAUDE_DESKTOP_ROUTES.len())
                .map(|m| {
                    (
                        m.display_name.clone(),
                        m.model.clone(),
                        m.context_window
                            .is_some_and(|w| w >= domain::CLAUDE_DESKTOP_ONE_M_WINDOW),
                    )
                })
                .collect(),
            _ => Vec::new(),
        };
        let desktop_roles: Vec<DesktopRoleDraft> = (0..domain::CLAUDE_DESKTOP_ROUTES.len())
            .map(|idx| {
                let (dn, m, one_m) = desktop_role_sources.get(idx).cloned().unwrap_or_default();
                let (role_label, _) = domain::CLAUDE_DESKTOP_ROUTES[idx];
                let is_haiku = role_label.eq_ignore_ascii_case("haiku");
                let dn_placeholder = if is_haiku {
                    "DeepSeek V4 Flash"
                } else {
                    "DeepSeek V4 Pro"
                };
                let model_placeholder = if is_haiku {
                    "deepseek-v4-flash"
                } else {
                    "deepseek-v4-pro"
                };
                DesktopRoleDraft {
                    display_name: field(window, cx, &dn, dn_placeholder),
                    model: field(window, cx, &m, model_placeholder),
                    one_m,
                    model_select: None,
                    _model_select_sub: None,
                }
            })
            .collect();

        // 接入方式下拉(对齐 cc-switch: 始终位于卡片右侧)
        let (desktop_mode_select, _desktop_mode_sub) = if app == AppKind::ClaudeDesktop {
            let modes = [
                (
                    domain::CLAUDE_DESKTOP_MODE_DIRECT,
                    t!("claude_desktop.model_mode_direct").to_string(),
                ),
                (
                    domain::CLAUDE_DESKTOP_MODE_MAPPING,
                    t!("claude_desktop.model_mode_mapping").to_string(),
                ),
            ];
            let items: Vec<DesktopModeItem> = modes
                .iter()
                .map(|(value, label)| DesktopModeItem {
                    value: value.to_string(),
                    label: label.to_string(),
                })
                .collect();
            let selected = modes
                .iter()
                .position(|(value, _)| *value == desktop_mode)
                .map(|i| gpui_component::IndexPath::default().row(i));
            let select = cx.new(|cx| SelectState::new(items, selected, window, cx));
            let view = cx.entity();
            let sub = window.subscribe(
                &select,
                cx,
                move |_, event: &SelectEvent<Vec<DesktopModeItem>>, _window, cx| {
                    if let SelectEvent::Confirm(Some(mode)) = event {
                        let mode = mode.clone();
                        view.update(cx, |this, cx| {
                            if let Some(form) = this.form.as_mut() {
                                form.desktop_mode = mode;
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

        let protocol_items = protocol_items();
        let protocol_idx = protocol_items
            .iter()
            .position(|item| item.value == protocol)
            .map(|i| gpui_component::IndexPath::default().row(i));
        let protocol_select =
            cx.new(|cx| SelectState::new(protocol_items, protocol_idx, window, cx));

        let (thinking_effort_select, _thinking_effort_sub) = if app == AppKind::Cursor {
            let options = thinking_effort_items();
            let idx = options
                .iter()
                .position(|item| item.value == thinking_effort)
                .or(Some(2))
                .map(|i| gpui_component::IndexPath::default().row(i));
            let select = cx.new(|cx| SelectState::new(options, idx, window, cx));
            let view = cx.entity();
            let sub = window.subscribe(
                &select,
                cx,
                move |_, event: &SelectEvent<Vec<WorkBuddyReasoningEffortItem>>, _window, cx| {
                    if let SelectEvent::Confirm(Some(value)) = event {
                        let effort = value.clone();
                        view.update(cx, |this, cx| {
                            if let Some(form) = this.form.as_mut() {
                                form.thinking_effort = effort;
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
            name: field(window, cx, &name, "输入服务商名称，如 OpenAI Official"),
            notes: field(window, cx, "", "例如：公司专用账号"),
            website_url: field(window, cx, &website_url, "https://chatgpt.com/codex"),
            codex_auth_mode: "follow".to_string(),
            codex_auth_dropdown_open: false,
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
            desktop_mode,
            desktop_mode_select,
            _desktop_mode_sub,
            desktop_roles,
            fetched_models: Vec::new(),
            has_fetched_models: false,
            default_model_select: None,
            is_fetching_models: false,
            is_testing_connectivity: false,
            connectivity_result: None,
            protocol_select,
            _protocol_sub: None,
            thinking_effort,
            thinking_effort_select,
            _thinking_effort_sub,
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
        let website_url = self.website_url.read(cx).value().to_string();
        let api_key = self.api_key.read(cx).value().to_string();
        let base_url = self.base_url.read(cx).value().to_string();
        let model = self.model.read(cx).value().to_string();
        let request_protocol = self
            .protocol_select
            .read(cx)
            .selected_value()
            .cloned()
            .unwrap_or_else(|| {
                RequestProtocol::default_for_app(self.app)
                    .as_str()
                    .to_string()
            });
        let default_reasoning_effort = if self.thinking_effort.trim().is_empty() {
            DEFAULT_THINKING_EFFORT.to_string()
        } else {
            self.thinking_effort.clone()
        };

        match self.app {
            AppKind::Codex => {
                let model_mappings = self
                    .catalog_rows
                    .iter()
                    .filter_map(|row| row.to_codex_mapping(cx))
                    .collect();
                ProviderForm::Codex(CodexForm {
                    name,
                    website_url,
                    kind: if self.is_official {
                        CodexKind::Official
                    } else {
                        CodexKind::ResponsesThirdParty
                    },
                    api_key,
                    base_url,
                    model,
                    request_protocol,
                    model_mappings,
                })
            }
            AppKind::Claude | AppKind::ClaudeDesktop => {
                // Claude Desktop 模型映射模式: 固定四档角色 -> model_mappings
                // (context_window >= 1M 表示声明支持 1M)；直连模式沿用通用映射
                let is_desktop_mapping = self.app == AppKind::ClaudeDesktop
                    && self.desktop_mode == domain::CLAUDE_DESKTOP_MODE_MAPPING;
                let model_mappings = if is_desktop_mapping {
                    let first_filled = self
                        .desktop_roles
                        .iter()
                        .find(|role| !role.model.read(cx).value().trim().is_empty());
                    if let Some(primary) = first_filled {
                        let primary_model = primary.model.read(cx).value().trim().to_string();
                        let primary_dn = primary.display_name.read(cx).value().trim().to_string();
                        let primary_1m = primary.one_m;
                        self.desktop_roles
                            .iter()
                            .map(|role| {
                                let m = role.model.read(cx).value().trim().to_string();
                                let dn = role.display_name.read(cx).value().trim().to_string();
                                let model = if m.is_empty() {
                                    primary_model.clone()
                                } else {
                                    m
                                };
                                let display_name = if dn.is_empty() {
                                    if !primary_dn.is_empty() {
                                        primary_dn.clone()
                                    } else {
                                        model.clone()
                                    }
                                } else {
                                    dn
                                };
                                let one_m = if role.one_m {
                                    true
                                } else if role.model.read(cx).value().trim().is_empty() {
                                    primary_1m
                                } else {
                                    false
                                };
                                domain::ClaudeModelMapping {
                                    display_name,
                                    model,
                                    context_window: one_m
                                        .then_some(domain::CLAUDE_DESKTOP_ONE_M_WINDOW),
                                    reasoning_effort: None,
                                }
                            })
                            .collect()
                    } else {
                        Vec::new()
                    }
                } else {
                    self.catalog_rows
                        .iter()
                        .filter_map(|row| row.to_claude_mapping(cx))
                        .collect()
                };
                let desktop_mode =
                    (self.app == AppKind::ClaudeDesktop).then(|| self.desktop_mode.clone());
                ProviderForm::Claude(ClaudeForm {
                    name,
                    website_url,
                    kind: if self.is_official {
                        ClaudeKind::Official
                    } else {
                        ClaudeKind::ThirdParty
                    },
                    api_key,
                    base_url,
                    model,
                    request_protocol,
                    model_mappings,
                    desktop_mode,
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
                    website_url,
                    kind: if self.is_official {
                        GrokKind::Official
                    } else {
                        GrokKind::ThirdParty
                    },
                    api_key,
                    base_url,
                    model,
                    request_protocol,
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
                    website_url,
                    kind: if self.is_official {
                        OpenCodeKind::Official
                    } else {
                        OpenCodeKind::ThirdParty
                    },
                    npm: domain::DEFAULT_OPENCODE_NPM.to_string(),
                    api_key,
                    base_url,
                    model,
                    request_protocol,
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
                    website_url,
                    kind: if self.is_official {
                        PiKind::Official
                    } else {
                        PiKind::ThirdParty
                    },
                    api_type: domain::DEFAULT_PI_API_TYPE.to_string(),
                    api_key,
                    base_url,
                    model,
                    request_protocol,
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
                    website_url,
                    kind: if self.is_official {
                        CursorKind::Official
                    } else {
                        CursorKind::ThirdParty
                    },
                    provider_type: request_protocol,
                    default_reasoning_effort,
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
                    website_url,
                    kind: if self.is_official {
                        ZCodeKind::Official
                    } else {
                        ZCodeKind::ThirdParty
                    },
                    provider_kind: DEFAULT_ZCODE_PROVIDER_KIND.to_string(),
                    api_key,
                    base_url,
                    model,
                    request_protocol,
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
                    website_url,
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

/// 表单字段标签
fn field_label(label: String, cx: &App) -> impl IntoElement {
    div()
        .text_size(px(12.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(cx.theme().foreground)
        .child(label)
}

/// 相对时间文案(卡片用量块)
fn relative_time_text(seconds_ago: i64, language: AppLanguage) -> String {
    let seconds = seconds_ago.max(0);
    if language == AppLanguage::En {
        if seconds < 60 {
            "just now".to_string()
        } else if seconds < 3600 {
            format!("{}m ago", seconds / 60)
        } else if seconds < 86400 {
            format!("{}h ago", seconds / 3600)
        } else {
            format!("{}d ago", seconds / 86400)
        }
    } else if seconds < 60 {
        "刚刚".to_string()
    } else if seconds < 3600 {
        format!("{} 分钟前", seconds / 60)
    } else if seconds < 86400 {
        format!("{} 小时前", seconds / 3600)
    } else {
        format!("{} 天前", seconds / 86400)
    }
}

/// 卡片用量块的余额行: "剩余: 6.90 USD" / 套餐名 / 查询失败
fn usage_value_line(
    result: &domain::UsageQueryResult,
    muted_foreground: gpui::Hsla,
) -> impl gpui::IntoElement {
    if !result.success {
        return h_flex()
            .items_center()
            .gap(px(4.))
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(muted_foreground)
                    .child("剩余:"),
            )
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(rgb(0xDC2626))
                    .child("查询失败"),
            )
            .into_any_element();
    }
    let Some(item) = result.data.first() else {
        return h_flex()
            .items_center()
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(muted_foreground)
                    .child("已启用"),
            )
            .into_any_element();
    };
    let unit = item.unit.clone().unwrap_or_else(|| "USD".into());
    h_flex()
        .items_center()
        .gap(px(4.))
        .child(
            div()
                .text_size(px(11.))
                .text_color(muted_foreground)
                .child("剩余:"),
        )
        .when_some(item.remaining, |this, remaining| {
            this.child(
                div()
                    .text_size(px(11.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(0x16A34A))
                    .child(format!("{remaining:.2}")),
            )
        })
        .children(item.plan_name.clone().map(|name| {
            div()
                .text_size(px(11.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(0x16A34A))
                .child(name)
        }))
        .child(
            div()
                .text_size(px(11.))
                .text_color(muted_foreground)
                .child(unit),
        )
        .into_any_element()
}

/// 用量结果的简短摘要(通知用)
fn usage_summary_text(result: &domain::UsageQueryResult) -> String {
    match result.data.first() {
        Some(item) => {
            let unit = item.unit.clone().unwrap_or_else(|| "USD".into());
            let mut text = if result.data.len() > 1 {
                format!("共 {} 个套餐: ", result.data.len())
            } else {
                String::new()
            };
            if let Some(name) = &item.plan_name {
                text.push_str(name);
                text.push_str(" ");
            }
            match (item.remaining, item.total) {
                (Some(remaining), Some(total)) => {
                    text.push_str(&format!("剩余 {remaining:.2}/{total:.2} {unit}"));
                }
                (Some(remaining), None) => {
                    text.push_str(&format!("剩余 {remaining:.2} {unit}"));
                }
                _ => text.push_str("查询成功"),
            }
            text
        }
        None => "查询成功".to_string(),
    }
}

fn notify_info(message: impl Into<SharedString>, window: &mut Window, cx: &mut App) {
    window.push_notification(Notification::info(message), cx);
}

/// 统一会话迁移/还原结果的提示文案
fn unify_outcome_message(
    outcome: &session::codex_history::HistoryOutcome,
    enabling: bool,
) -> String {
    if let Some(reason) = &outcome.skipped_reason {
        return match reason.as_str() {
            "unify_toggle_on" => t!("codex_enhance.restore_skipped_toggle_on").to_string(),
            "no_backup_ledger" => t!("codex_enhance.restore_nothing").to_string(),
            "nothing_to_restore" => t!("codex_enhance.restore_nothing").to_string(),
            "live_not_unified" => t!("codex_enhance.migrate_skipped_live_not_unified").to_string(),
            "already_migrated" => t!("codex_enhance.migrate_already_done").to_string(),
            _ => {
                if enabling {
                    t!("codex_enhance.enable_saved").to_string()
                } else {
                    t!("codex_enhance.disable_saved").to_string()
                }
            }
        };
    }
    t!(
        if enabling {
            "codex_enhance.migrate_completed"
        } else {
            "codex_enhance.restore_completed"
        },
        files = outcome.jsonl_files,
        rows = outcome.state_rows
    )
    .to_string()
}

fn notify_error(message: impl Into<SharedString>, window: &mut Window, cx: &mut App) {
    window.push_notification(Notification::error(message), cx);
}

/// 检查更新结果对话框: 列出有更新的受管技能
fn open_skills_updates_dialog(
    window: &mut Window,
    cx: &mut App,
    updates: Vec<session::skills::hub::SkillUpdateInfo>,
    on_update: impl Fn(String, &mut Window, &mut App) + Clone + 'static,
) {
    let theme = cx.theme().clone();
    window.open_dialog(cx, move |dialog, _window, _cx| {
        let theme = theme.clone();
        let on_update = on_update.clone();
        let rows: Vec<_> = updates
            .iter()
            .map(|info| {
                let theme = theme.clone();
                let on_update = on_update.clone();
                let dir_name = info.dir_name.clone();
                let btn_id = SharedString::from(format!("skill-update-{}", info.dir_name));
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .gap(px(12.))
                    .p(px(10.))
                    .rounded(px(8.))
                    .border_1()
                    .border_color(theme.border)
                    .child(
                        v_flex()
                            .min_w_0()
                            .gap(px(2.))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(theme.foreground)
                                    .child(info.name.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .text_color(theme.muted_foreground)
                                    .child(format!("{} · {}", info.repo, info.dir_name)),
                            ),
                    )
                    .child(
                        Button::new(btn_id)
                            .primary()
                            .xsmall()
                            .label(t!("skills.update_action").to_string())
                            .on_click(move |_, window, cx| {
                                window.close_dialog(cx);
                                on_update(dir_name.clone(), window, cx);
                            }),
                    )
                    .into_any_element()
            })
            .collect();
        dialog
            .width(px(520.))
            .title(t!("skills.updates_title", count = updates.len()).to_string())
            .child(
                v_flex()
                    .id("skills-updates-list")
                    .w_full()
                    .max_h(px(400.))
                    .overflow_y_scroll()
                    .gap(px(8.))
                    .children(rows),
            )
    });
}

/// 备份列表中的一行
fn render_backup_row(
    entry: &session::skills::hub::SkillBackupEntry,
    view: gpui::WeakEntity<RouterApp>,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme();
    let time = chrono::DateTime::from_timestamp_millis(entry.created_at)
        .map(|t| {
            t.with_timezone(&chrono::Local)
                .format("%Y-%m-%d %H:%M")
                .to_string()
        })
        .unwrap_or_default();
    let title = entry.name.clone().unwrap_or_else(|| entry.dir_name.clone());
    let v_restore = view.clone();
    let entry_restore = entry.clone();
    let v_delete = view.clone();
    let entry_delete = entry.clone();
    let restore_id = SharedString::from(format!("backup-restore-{}", entry.backup_id));
    let delete_id = SharedString::from(format!("backup-delete-{}", entry.backup_id));
    h_flex()
        .w_full()
        .items_center()
        .justify_between()
        .gap(px(12.))
        .p(px(10.))
        .rounded(px(8.))
        .border_1()
        .border_color(theme.border)
        .child(
            v_flex()
                .min_w_0()
                .gap(px(2.))
                .child(
                    div()
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(theme.foreground)
                        .child(title),
                )
                .child(
                    div()
                        .text_size(px(11.5))
                        .text_color(theme.muted_foreground)
                        .child(format!(
                            "{time} · {} · {}",
                            t!("skills.backup_origin", origin = entry.origin.as_str()).to_string(),
                            entry.source_path
                        )),
                ),
        )
        .child(
            h_flex()
                .items_center()
                .gap(px(6.))
                .child(
                    Button::new(restore_id)
                        .outline()
                        .xsmall()
                        .label(t!("skills.backup_restore").to_string())
                        .on_click(move |_, window, cx| {
                            let _ = v_restore.update(cx, |this, cx| {
                                this.restore_skill_backup(entry_restore.clone(), window, cx)
                            });
                        }),
                )
                .child(
                    Button::new(delete_id)
                        .ghost()
                        .xsmall()
                        .label(t!("skills.backup_delete").to_string())
                        .on_click(move |_, window, cx| {
                            let _ = v_delete.update(cx, |this, cx| {
                                this.confirm_delete_backup(
                                    entry_delete.backup_id.clone(),
                                    window,
                                    cx,
                                )
                            });
                        }),
                ),
        )
}

/// 导入已有对话框中的一行
fn render_import_row(
    item: &SkillsUnmanaged,
    selected: bool,
    view: gpui::WeakEntity<RouterApp>,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme();
    let title = item.name.clone().unwrap_or_else(|| item.dir_name.clone());
    let subtitle = format!(
        "{} · {}",
        item.app,
        item.description.clone().unwrap_or_default()
    );
    let dir_name = item.dir_name.clone();
    h_flex()
        .w_full()
        .items_center()
        .gap(px(10.))
        .p(px(10.))
        .rounded(px(8.))
        .border_1()
        .border_color(theme.border)
        .child(
            Checkbox::new(SharedString::from(format!(
                "import-check-{}",
                item.dir_name
            )))
            .checked(selected)
            .on_click(move |checked: &bool, _window, cx| {
                let _ = view.update(cx, |this, cx| {
                    if *checked {
                        this.skills_import_selected.insert(dir_name.clone());
                    } else {
                        this.skills_import_selected.remove(&dir_name);
                    }
                    cx.notify();
                });
            }),
        )
        .child(
            v_flex()
                .min_w_0()
                .gap(px(2.))
                .child(
                    div()
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(theme.foreground)
                        .child(title),
                )
                .child(
                    div()
                        .text_size(px(11.5))
                        .text_color(theme.muted_foreground)
                        .truncate()
                        .child(subtitle),
                ),
        )
}

/// 仓库管理对话框中的一行
fn render_repo_row(
    repo: &session::skills::hub::SkillRepo,
    view: gpui::WeakEntity<RouterApp>,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme();
    let label = format!("{} ({})", repo.key(), repo.branch);
    let v_toggle = view.clone();
    let (owner_t, name_t) = (repo.owner.clone(), repo.name.clone());
    let enabled = repo.enabled;
    let v_remove = view.clone();
    let (owner_r, name_r) = (repo.owner.clone(), repo.name.clone());
    let remove_id = SharedString::from(format!("repo-remove-{}-{}", repo.owner, repo.name));
    h_flex()
        .w_full()
        .items_center()
        .gap(px(10.))
        .p(px(10.))
        .rounded(px(8.))
        .border_1()
        .border_color(theme.border)
        .child(
            Checkbox::new(SharedString::from(format!(
                "repo-enable-{}-{}",
                repo.owner, repo.name
            )))
            .checked(enabled)
            .on_click(move |checked: &bool, _window, cx| {
                let v = v_toggle.clone();
                let (o, n) = (owner_t.clone(), name_t.clone());
                let checked = *checked;
                let _ = v.update(cx, |this, cx| {
                    this.toggle_skill_repo_enabled(o, n, checked, cx);
                });
            }),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_size(px(13.))
                .text_color(theme.foreground)
                .truncate()
                .child(label),
        )
        .child(
            Button::new(remove_id)
                .ghost()
                .xsmall()
                .label(t!("skills.repo_remove").to_string())
                .on_click(move |_, _window, cx| {
                    let v = v_remove.clone();
                    let (o, n) = (owner_r.clone(), name_r.clone());
                    let _ = v.update(cx, |this, cx| this.remove_skill_repo(o, n, cx));
                }),
        )
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
