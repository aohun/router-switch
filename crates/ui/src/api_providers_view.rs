//! AstrLink-aligned API Providers page (gateway upstream registry).
//! Distinct from per-app Live `Route::Codex` / Claude / … provider pages.

use domain::{ApiProvider, ApiProviderKind, ApiProviderKindGroup, HttpConnection};
use gpui::{
    div, prelude::FluentBuilder, px, rgb, App, AppContext, Context, Entity, FontWeight, Hsla,
    InteractiveElement, IntoElement, ParentElement, SharedString, StatefulInteractiveElement,
    Styled, Window,
};
use gpui_component::{
    button::{Button, ButtonCustomVariant, ButtonVariants},
    checkbox::Checkbox,
    h_flex,
    input::{Input, InputState},
    scroll::ScrollableElement as _,
    v_flex, ActiveTheme, Disableable, Icon, IconName, Sizable,
};
use rust_i18n::t;

use crate::app_view::RouterApp;
use crate::assets::brand_img;

/// AstrLink light-mode `--primary-fill` / `--primary-hover`.
const ASTRLINK_PRIMARY: u32 = 0x1D4D87;
const ASTRLINK_PRIMARY_HOVER: u32 = 0x16345C;
const ASTRLINK_PRIMARY_ACTIVE: u32 = 0x122B4C;

fn astrlink_primary_btn(cx: &App) -> ButtonCustomVariant {
    ButtonCustomVariant::new(cx)
        .color(Hsla::from(rgb(ASTRLINK_PRIMARY)))
        .foreground(Hsla::from(rgb(0xFFFFFF)))
        .border(Hsla::from(rgb(ASTRLINK_PRIMARY)))
        .hover(Hsla::from(rgb(ASTRLINK_PRIMARY_HOVER)))
        .active(Hsla::from(rgb(ASTRLINK_PRIMARY_ACTIVE)))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ApiProvidersFilter {
    #[default]
    All,
    Enabled,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ApiProvidersPane {
    #[default]
    List,
    Create,
    Edit(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ApiProvidersEditTab {
    #[default]
    Connection,
    Models,
    Protocols,
    Failure,
}

/// AstrLink authorization flow for subscription kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ApiAuthFlow {
    #[default]
    Browser,
    DeviceCode,
    AuthorizationCode,
}

pub struct ApiProvidersState {
    pub items: Vec<ApiProvider>,
    pub loading: bool,
    pub pane: ApiProvidersPane,
    pub filter: ApiProvidersFilter,
    pub edit_tab: ApiProvidersEditTab,
    pub draft: Option<ApiProvider>,
    pub auth_flow: ApiAuthFlow,
    pub kind_menu_open: bool,
    pub search: Entity<InputState>,
    pub model_filter: Entity<InputState>,
    pub draft_name: Entity<InputState>,
    pub draft_base_url: Entity<InputState>,
    pub draft_api_key: Entity<InputState>,
    /// Legacy multiline sync; models tab now mutates `draft.models` directly.
    pub draft_models: Entity<InputState>,
    pub models_query: Entity<InputState>,
    pub models_add_input: Entity<InputState>,
    pub models_adding: bool,
    pub models_bulk: bool,
    pub models_fetching: bool,
    pub status_message: Option<String>,
}

impl ApiProvidersState {
    pub fn new(window: &mut Window, cx: &mut Context<RouterApp>) -> Self {
        Self {
            items: Vec::new(),
            loading: false,
            pane: ApiProvidersPane::List,
            filter: ApiProvidersFilter::All,
            edit_tab: ApiProvidersEditTab::Connection,
            draft: None,
            auth_flow: ApiAuthFlow::Browser,
            kind_menu_open: false,
            search: cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder(t!("api_providers.search_placeholder").to_string())
            }),
            model_filter: cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder(t!("api_providers.filter_model").to_string())
            }),
            draft_name: cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder(t!("api_providers.name_placeholder").to_string())
            }),
            draft_base_url: cx
                .new(|cx| InputState::new(window, cx).placeholder("https://api.example.com/v1")),
            draft_api_key: cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder(t!("api_providers.api_key_placeholder").to_string())
                    .masked(true)
            }),
            draft_models: cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder(t!("api_providers.models_placeholder").to_string())
            }),
            models_query: cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder(t!("api_providers.models_search").to_string())
            }),
            models_add_input: cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder(t!("api_providers.models_add_placeholder").to_string())
            }),
            models_adding: false,
            models_bulk: false,
            models_fetching: false,
            status_message: None,
        }
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn zh_locale() -> bool {
    rust_i18n::locale().starts_with("zh")
}

fn kind_group_label(group: ApiProviderKindGroup) -> String {
    match group {
        ApiProviderKindGroup::Subscription => t!("api_providers.group_subscription").to_string(),
        ApiProviderKindGroup::Gateway => t!("api_providers.group_gateway").to_string(),
        ApiProviderKindGroup::PayAsYouGo => t!("api_providers.group_payg").to_string(),
        ApiProviderKindGroup::Advanced => t!("api_providers.group_advanced").to_string(),
    }
}

fn group_key(group: ApiProviderKindGroup) -> &'static str {
    match group {
        ApiProviderKindGroup::Subscription => "subscription",
        ApiProviderKindGroup::Gateway => "gateway",
        ApiProviderKindGroup::PayAsYouGo => "payg",
        ApiProviderKindGroup::Advanced => "advanced",
    }
}

fn parse_model_ids(raw: &str) -> Vec<String> {
    let mut out = Vec::new();
    for part in raw.split(|c: char| c == '\n' || c == ',' || c == ';' || c == ' ') {
        let id = part.trim();
        if !id.is_empty() && !out.iter().any(|x: &String| x == id) {
            out.push(id.to_string());
        }
    }
    out
}

fn filter_model_ids(models: &[String], query: &str) -> Vec<String> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        let mut sorted = models.to_vec();
        sorted.sort();
        return sorted;
    }
    let mut filtered: Vec<String> = models
        .iter()
        .filter(|m| m.to_lowercase().contains(&q))
        .cloned()
        .collect();
    filtered.sort();
    filtered
}

impl RouterApp {
    pub(crate) fn refresh_api_providers(&mut self, cx: &mut Context<Self>) {
        self.api_providers.loading = true;
        cx.notify();
        match self.workspace.list_api_providers() {
            Ok(items) => {
                self.api_providers.items = items;
                self.api_providers.status_message = None;
            }
            Err(err) => {
                self.api_providers.status_message = Some(err.to_string());
            }
        }
        self.api_providers.loading = false;
        cx.notify();
    }

    pub(crate) fn open_api_providers_create(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let kind = ApiProviderKind::CodexSubscription;
        let draft = ApiProvider::new_from_kind(
            kind,
            if zh_locale() {
                "Codex 订阅"
            } else {
                "Codex Subscription"
            },
            now_ms(),
        );
        self.sync_draft_inputs(&draft, window, cx);
        self.api_providers.auth_flow = ApiAuthFlow::Browser;
        self.api_providers.kind_menu_open = false;
        self.api_providers.models_adding = false;
        self.api_providers.models_bulk = false;
        self.api_providers.draft = Some(draft);
        self.api_providers.edit_tab = ApiProvidersEditTab::Connection;
        self.api_providers.pane = ApiProvidersPane::Create;
        cx.notify();
    }

    pub(crate) fn open_api_providers_edit(
        &mut self,
        id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(p) = self
            .api_providers
            .items
            .iter()
            .find(|x| x.id == id)
            .cloned()
        else {
            return;
        };
        self.sync_draft_inputs(&p, window, cx);
        self.api_providers.draft = Some(p.clone());
        self.api_providers.kind_menu_open = false;
        self.api_providers.models_adding = false;
        self.api_providers.models_bulk = false;
        self.api_providers.edit_tab = ApiProvidersEditTab::Connection;
        self.api_providers.pane = ApiProvidersPane::Edit(id.to_string());
        cx.notify();
    }

    fn sync_draft_inputs(&mut self, p: &ApiProvider, window: &mut Window, cx: &mut Context<Self>) {
        self.api_providers.draft_name.update(cx, |input, cx| {
            input.set_value(p.name.clone(), window, cx);
        });
        let base = p.http.as_ref().map(|h| h.base_url.as_str()).unwrap_or("");
        self.api_providers.draft_base_url.update(cx, |input, cx| {
            input.set_value(base.to_string(), window, cx);
        });
        let key = p
            .http
            .as_ref()
            .and_then(|h| h.api_key.as_deref())
            .unwrap_or("");
        self.api_providers.draft_api_key.update(cx, |input, cx| {
            input.set_value(key.to_string(), window, cx);
        });
        self.api_providers.draft_models.update(cx, |input, cx| {
            input.set_value(p.models.join("\n"), window, cx);
        });
    }

    pub(crate) fn save_api_provider_draft(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(mut draft) = self.api_providers.draft.clone() else {
            return;
        };
        let name = self.api_providers.draft_name.read(cx).value().to_string();
        let name = name.trim().to_string();
        if name.is_empty() {
            self.api_providers.status_message =
                Some(t!("api_providers.err_name_required").to_string());
            cx.notify();
            return;
        }
        draft.name = name;
        if !draft.kind.is_subscription() {
            let base = self
                .api_providers
                .draft_base_url
                .read(cx)
                .value()
                .to_string();
            let key = self
                .api_providers
                .draft_api_key
                .read(cx)
                .value()
                .to_string();
            let mut http = draft.http.unwrap_or_else(|| HttpConnection {
                base_url: String::new(),
                auth_scheme: draft.kind.default_auth_scheme(),
                header_name: None,
                api_key: None,
            });
            http.base_url = base.trim().to_string();
            let key = key.trim().to_string();
            http.api_key = if key.is_empty() { None } else { Some(key) };
            draft.http = Some(http);
        }
        // Models tab mutates draft.models in place; keep that allow-list.
        draft.updated_at = now_ms();
        match self.workspace.upsert_api_provider(&draft) {
            Ok(()) => {
                self.api_providers.pane = ApiProvidersPane::List;
                self.api_providers.draft = None;
                self.api_providers.status_message = Some(t!("api_providers.saved").to_string());
                self.refresh_api_providers(cx);
            }
            Err(err) => {
                self.api_providers.status_message = Some(err.to_string());
                cx.notify();
            }
        }
        let _ = window;
    }

    pub(crate) fn toggle_api_provider_enabled(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(current) = self.api_providers.items.iter().find(|p| p.id == id) else {
            return;
        };
        let enabled = !current.enabled;
        match self.workspace.set_api_provider_enabled(id, enabled) {
            Ok(_) => self.refresh_api_providers(cx),
            Err(err) => {
                self.api_providers.status_message = Some(err.to_string());
                cx.notify();
            }
        }
    }

    pub(crate) fn delete_api_provider(&mut self, id: &str, cx: &mut Context<Self>) {
        match self.workspace.delete_api_provider(id) {
            Ok(()) => {
                if matches!(self.api_providers.pane, ApiProvidersPane::Edit(ref e) if e == id) {
                    self.api_providers.pane = ApiProvidersPane::List;
                    self.api_providers.draft = None;
                }
                self.refresh_api_providers(cx);
            }
            Err(err) => {
                self.api_providers.status_message = Some(err.to_string());
                cx.notify();
            }
        }
    }

    pub(crate) fn set_draft_kind(
        &mut self,
        kind: ApiProviderKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !matches!(self.api_providers.pane, ApiProvidersPane::Create) {
            return;
        }
        let name = if zh_locale() {
            kind.display_zh()
        } else {
            kind.display_en()
        };
        let mut draft = ApiProvider::new_from_kind(kind, name, now_ms());
        if let Some(old) = &self.api_providers.draft {
            draft.enabled = old.enabled;
            draft.proxy_mode = old.proxy_mode.clone();
        }
        self.api_providers.auth_flow = match kind {
            ApiProviderKind::ClaudeSubscription => ApiAuthFlow::AuthorizationCode,
            ApiProviderKind::GrokSubscription => ApiAuthFlow::DeviceCode,
            _ => ApiAuthFlow::Browser,
        };
        self.api_providers.kind_menu_open = false;
        self.sync_draft_inputs(&draft, window, cx);
        self.api_providers.draft = Some(draft);
        cx.notify();
    }

    pub(crate) fn render_api_providers_page(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        match &self.api_providers.pane {
            ApiProvidersPane::List => self.render_api_providers_list(cx).into_any_element(),
            ApiProvidersPane::Create | ApiProvidersPane::Edit(_) => {
                self.render_api_providers_editor(cx).into_any_element()
            }
        }
    }

    fn filtered_api_providers(&self, cx: &Context<Self>) -> Vec<ApiProvider> {
        let q = self.api_providers.search.read(cx).value().to_string();
        let q = q.trim().to_lowercase();
        let model = self.api_providers.model_filter.read(cx).value().to_string();
        let model = model.trim().to_string();
        self.api_providers
            .items
            .iter()
            .filter(|p| match self.api_providers.filter {
                ApiProvidersFilter::All => true,
                ApiProvidersFilter::Enabled => p.enabled,
                ApiProvidersFilter::Disabled => !p.enabled,
            })
            .filter(|p| p.matches_query(&q))
            .filter(|p| p.matches_model_filter(&model))
            .cloned()
            .collect()
    }

    fn render_api_providers_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let all_n = self.api_providers.items.len();
        let enabled_n = self
            .api_providers
            .items
            .iter()
            .filter(|p| p.enabled)
            .count();
        let disabled_n = all_n.saturating_sub(enabled_n);
        let rows = self.filtered_api_providers(cx);
        let zh = zh_locale();

        v_flex()
            .size_full()
            .bg(theme.background)
            .child(
                // Header
                h_flex()
                    .w_full()
                    .px(px(24.))
                    .pt(px(20.))
                    .pb(px(12.))
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(22.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.foreground)
                            .child(t!("api_providers.title").to_string()),
                    )
                    .child(
                        h_flex()
                            .gap(px(8.))
                            .items_center()
                            .child(
                                Button::new("api-prov-help")
                                    .ghost()
                                    .icon(IconName::Info)
                                    .tooltip(t!("api_providers.order_help").to_string()),
                            )
                            .child(
                                Button::new("api-prov-refresh")
                                    .ghost()
                                    .icon(IconName::Redo)
                                    .tooltip(t!("api_providers.refresh").to_string())
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.refresh_api_providers(cx);
                                    })),
                            )
                            .child(
                                Button::new("api-prov-add")
                                    .custom(astrlink_primary_btn(cx))
                                    .icon(IconName::Plus)
                                    .label(t!("api_providers.add").to_string())
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.open_api_providers_create(window, cx);
                                    })),
                            ),
                    ),
            )
            .child(
                // Toolbar: status tabs + search + model filter
                h_flex()
                    .w_full()
                    .px(px(24.))
                    .pb(px(12.))
                    .gap(px(12.))
                    .items_center()
                    .child(self.render_status_tab(
                        "api-filter-all",
                        ApiProvidersFilter::All,
                        format!("{} {all_n}", t!("api_providers.filter_all")),
                        cx,
                    ))
                    .child(self.render_status_tab(
                        "api-filter-on",
                        ApiProvidersFilter::Enabled,
                        format!("{} {enabled_n}", t!("api_providers.filter_enabled")),
                        cx,
                    ))
                    .child(self.render_status_tab(
                        "api-filter-off",
                        ApiProvidersFilter::Disabled,
                        format!("{} {disabled_n}", t!("api_providers.filter_disabled")),
                        cx,
                    ))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(160.))
                            .child(Input::new(&self.api_providers.search).cleanable(true)),
                    )
                    .child(
                        div()
                            .w(px(200.))
                            .child(Input::new(&self.api_providers.model_filter).cleanable(true)),
                    ),
            )
            .when_some(self.api_providers.status_message.clone(), |el, msg| {
                el.child(
                    div()
                        .px(px(24.))
                        .pb(px(8.))
                        .text_size(px(12.))
                        .text_color(theme.muted_foreground)
                        .child(msg),
                )
            })
            .child(if self.api_providers.loading {
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(theme.muted_foreground)
                    .child(t!("api_providers.loading").to_string())
                    .into_any_element()
            } else if self.api_providers.items.is_empty() {
                // AstrLink EmptyState: dashed panel + title + hint + primary CTA
                div()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .px(px(24.))
                    .pb(px(24.))
                    .child(
                        v_flex()
                            .size_full()
                            .items_center()
                            .justify_center()
                            .gap(px(6.))
                            .rounded(px(8.))
                            .border_1()
                            .border_dashed()
                            .border_color(theme.border)
                            .px(px(24.))
                            .py(px(40.))
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(theme.foreground)
                                    .child(t!("api_providers.empty").to_string()),
                            )
                            .child(
                                div()
                                    .max_w(px(420.))
                                    .text_size(px(12.))
                                    .text_color(theme.muted_foreground)
                                    .child(t!("api_providers.empty_hint").to_string()),
                            )
                            .child(
                                div().mt(px(6.)).child(
                                    Button::new("api-prov-empty-add")
                                        .custom(astrlink_primary_btn(cx))
                                        .icon(IconName::Plus)
                                        .label(t!("api_providers.add").to_string())
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.open_api_providers_create(window, cx);
                                        })),
                                ),
                            ),
                    )
                    .into_any_element()
            } else if rows.is_empty() {
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(theme.muted_foreground)
                    .text_size(px(13.))
                    .child(t!("api_providers.filter_empty").to_string())
                    .into_any_element()
            } else {
                v_flex()
                    .flex_1()
                    .min_h_0()
                    .child(
                        // Column headers (only when there is data)
                        h_flex()
                            .w_full()
                            .px(px(24.))
                            .py(px(8.))
                            .border_y_1()
                            .border_color(theme.border)
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.muted_foreground)
                            .child(div().w(px(36.)).child("#"))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w(px(180.))
                                    .child(t!("api_providers.col_service").to_string()),
                            )
                            .child(
                                div()
                                    .w(px(140.))
                                    .child(t!("api_providers.col_models").to_string()),
                            )
                            .child(
                                div()
                                    .w(px(120.))
                                    .child(t!("api_providers.col_usage").to_string()),
                            )
                            .child(
                                div()
                                    .w(px(100.))
                                    .child(t!("api_providers.col_billing").to_string()),
                            )
                            .child(
                                div()
                                    .w(px(80.))
                                    .child(t!("api_providers.col_status").to_string()),
                            )
                            .child(
                                div()
                                    .w(px(140.))
                                    .child(t!("api_providers.col_actions").to_string()),
                            ),
                    )
                    .child(
                        v_flex().flex_1().min_h_0().overflow_y_scrollbar().children(
                            rows.into_iter()
                                .enumerate()
                                .map(|(i, p)| self.render_api_provider_row(i + 1, p, zh, cx)),
                        ),
                    )
                    .into_any_element()
            })
    }

    fn render_status_tab(
        &self,
        id: &'static str,
        filter: ApiProvidersFilter,
        label: String,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme().clone();
        let selected = self.api_providers.filter == filter;
        div()
            .id(SharedString::from(id))
            .px(px(12.))
            .py(px(6.))
            .rounded(px(8.))
            .cursor_pointer()
            .text_size(px(13.))
            .when(selected, |el| {
                el.bg(rgb(0xE8F1FF))
                    .text_color(rgb(0x1D4ED8))
                    .font_weight(FontWeight::MEDIUM)
            })
            .when(!selected, |el| {
                el.text_color(theme.muted_foreground)
                    .hover(|s| s.bg(theme.secondary.opacity(0.6)))
            })
            .on_click(cx.listener(move |this, _, _, cx| {
                this.api_providers.filter = filter;
                cx.notify();
            }))
            .child(label)
    }

    fn render_api_provider_row(
        &self,
        index: usize,
        p: ApiProvider,
        zh: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme().clone();
        let id = p.id.clone();
        let id_edit = p.id.clone();
        let id_toggle = p.id.clone();
        let id_del = p.id.clone();
        let kind_label = p.kind_label(zh).to_string();
        let hint = p.connection_hint();
        let model_count = p.models.len();
        let cap_count = p.capabilities.len();

        h_flex()
            .id(SharedString::from(format!("api-row-{}", p.id)))
            .w_full()
            .px(px(24.))
            .py(px(14.))
            .gap(px(8.))
            .items_center()
            .border_b_1()
            .border_color(theme.border.opacity(0.6))
            .hover(|s| s.bg(theme.secondary.opacity(0.35)))
            .child(
                div()
                    .w(px(36.))
                    .text_size(px(12.))
                    .text_color(theme.muted_foreground)
                    .child(format!("{index}")),
            )
            .child(
                v_flex()
                    .flex_1()
                    .min_w(px(180.))
                    .gap(px(2.))
                    .child(
                        div()
                            .id(SharedString::from(format!("api-name-{}", p.id)))
                            .text_size(px(14.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.foreground)
                            .cursor_pointer()
                            .child(p.name.clone())
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.open_api_providers_edit(&id_edit, window, cx);
                            })),
                    )
                    .child(
                        h_flex()
                            .gap(px(8.))
                            .text_size(px(11.))
                            .text_color(theme.muted_foreground)
                            .child(kind_label)
                            .child(div().opacity(0.5).child("·"))
                            .child(
                                div()
                                    .max_w(px(280.))
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .child(hint),
                            ),
                    ),
            )
            .child(
                v_flex()
                    .w(px(140.))
                    .gap(px(2.))
                    .text_size(px(12.))
                    .text_color(theme.muted_foreground)
                    .child(format!(
                        "{} {}",
                        model_count,
                        t!("api_providers.models_unit")
                    ))
                    .child(format!("{} {}", cap_count, t!("api_providers.caps_unit"))),
            )
            .child(
                div()
                    .w(px(120.))
                    .text_size(px(12.))
                    .text_color(theme.muted_foreground)
                    .child(
                        if p.kind.is_subscription()
                            || matches!(
                                p.kind,
                                ApiProviderKind::OpencodeGo
                                    | ApiProviderKind::KimiCoding
                                    | ApiProviderKind::GlmCoding
                                    | ApiProviderKind::MinimaxCoding
                                    | ApiProviderKind::Newapi
                            )
                        {
                            t!("api_providers.usage_na").to_string()
                        } else {
                            String::new()
                        },
                    ),
            )
            .child(
                div()
                    .w(px(100.))
                    .text_size(px(12.))
                    .text_color(theme.muted_foreground)
                    .child(t!("api_providers.billing_month_zero").to_string()),
            )
            .child(
                // Enable switch (clickable pill)
                div()
                    .id(SharedString::from(format!("api-sw-{}", p.id)))
                    .w(px(80.))
                    .cursor_pointer()
                    .child(
                        h_flex()
                            .w(px(40.))
                            .h(px(22.))
                            .rounded(px(11.))
                            .px(px(2.))
                            .items_center()
                            .when(p.enabled, |el| el.bg(rgb(0x2563EB)).justify_end())
                            .when(!p.enabled, |el| el.bg(rgb(0xD1D5DB)).justify_start())
                            .child(div().size(px(18.)).rounded_full().bg(rgb(0xFFFFFF))),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.toggle_api_provider_enabled(&id_toggle, cx);
                    })),
            )
            .child(
                h_flex()
                    .w(px(140.))
                    .gap(px(4.))
                    .child(
                        Button::new(SharedString::from(format!("api-test-{}", p.id)))
                            .ghost()
                            .xsmall()
                            .icon(IconName::Inspector)
                            .tooltip(t!("api_providers.test").to_string())
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.api_providers.status_message =
                                    Some(t!("api_providers.test_todo").to_string());
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new(SharedString::from(format!("api-edit-{}", p.id)))
                            .ghost()
                            .xsmall()
                            .icon(IconName::Settings2)
                            .tooltip(t!("api_providers.edit").to_string())
                            .on_click(cx.listener({
                                let id = id.clone();
                                move |this, _, window, cx| {
                                    this.open_api_providers_edit(&id, window, cx);
                                }
                            })),
                    )
                    .child(
                        Button::new(SharedString::from(format!("api-del-{}", p.id)))
                            .ghost()
                            .xsmall()
                            .icon(IconName::Delete)
                            .tooltip(t!("api_providers.delete").to_string())
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.delete_api_provider(&id_del, cx);
                            })),
                    ),
            )
    }

    fn render_api_providers_editor(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let creating = matches!(self.api_providers.pane, ApiProvidersPane::Create);
        let draft = self.api_providers.draft.clone();
        let kind = draft.as_ref().map(|d| d.kind);
        let is_sub = kind.map(|k| k.is_subscription()).unwrap_or(false);
        let tab = self.api_providers.edit_tab;
        let title = if creating {
            t!("api_providers.add").to_string()
        } else {
            t!("api_providers.edit_title").to_string()
        };
        let save_label = if creating && is_sub {
            t!("api_providers.add_and_login").to_string()
        } else if creating {
            t!("api_providers.save_service").to_string()
        } else {
            t!("api_providers.save_changes").to_string()
        };

        v_flex()
            .size_full()
            .bg(theme.secondary.opacity(0.35))
            // AstrLink PageHeader compact: back + / + title, then border-b
            .child(
                h_flex()
                    .w_full()
                    .px(px(24.))
                    .pt(px(12.))
                    .pb(px(10.))
                    .gap(px(8.))
                    .items_center()
                    .border_b_1()
                    .border_color(theme.border)
                    .child(
                        // px-0 link style so the arrow lines up with cards below
                        h_flex()
                            .id("api-ed-back")
                            .gap(px(4.))
                            .items_center()
                            .cursor_pointer()
                            .text_size(px(12.))
                            .text_color(theme.muted_foreground)
                            .hover(|s| s.text_color(theme.foreground))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.api_providers.pane = ApiProvidersPane::List;
                                this.api_providers.draft = None;
                                this.api_providers.kind_menu_open = false;
                                cx.notify();
                            }))
                            .child(Icon::new(IconName::ArrowLeft).size(px(12.)))
                            .child(t!("api_providers.back").to_string()),
                    )
                    .child(div().text_size(px(13.)).text_color(theme.border).child("/"))
                    .child(
                        div()
                            .text_size(px(14.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.foreground)
                            .child(title),
                    ),
            )
            .child(
                // AstrLink TabsList: muted pill track, elevated active tab
                h_flex().w_full().px(px(24.)).pt(px(12.)).pb(px(12.)).child(
                    h_flex()
                        .items_center()
                        .rounded(px(8.))
                        .p(px(3.))
                        .bg(theme.secondary)
                        .gap(px(0.))
                        .child(self.render_edit_tab(
                            "tab-conn",
                            ApiProvidersEditTab::Connection,
                            IconName::Globe,
                            t!("api_providers.tab_connection").to_string(),
                            cx,
                        ))
                        .child(self.render_tab_divider(cx))
                        .child(self.render_edit_tab(
                            "tab-models",
                            ApiProvidersEditTab::Models,
                            IconName::Building2,
                            t!("api_providers.tab_models").to_string(),
                            cx,
                        ))
                        .child(self.render_tab_divider(cx))
                        .child(self.render_edit_tab(
                            "tab-proto",
                            ApiProvidersEditTab::Protocols,
                            IconName::Settings2,
                            t!("api_providers.tab_protocols").to_string(),
                            cx,
                        ))
                        .child(self.render_tab_divider(cx))
                        .child(self.render_edit_tab(
                            "tab-fail",
                            ApiProvidersEditTab::Failure,
                            IconName::TriangleAlert,
                            t!("api_providers.tab_failure").to_string(),
                            cx,
                        )),
                ),
            )
            .child({
                // Freeze page scroll while kind menu is open so the menu can scroll
                // through gateway / pay-as-you-go / advanced groups.
                let menu_open = self.api_providers.kind_menu_open;
                let body = v_flex()
                    .flex_1()
                    .min_h_0()
                    .px(px(24.))
                    .pb(px(12.))
                    .gap(px(16.))
                    .when(tab == ApiProvidersEditTab::Connection, |el| {
                        el.child(self.render_connection_tab(creating, is_sub, kind, cx))
                    })
                    .when(tab == ApiProvidersEditTab::Models, |el| {
                        el.child(self.render_models_tab(cx))
                    })
                    .when(tab == ApiProvidersEditTab::Protocols, |el| {
                        el.child(self.render_protocols_tab(cx))
                    })
                    .when(tab == ApiProvidersEditTab::Failure, |el| {
                        el.child(
                            div()
                                .p(px(16.))
                                .rounded(px(10.))
                                .border_1()
                                .border_color(theme.border)
                                .bg(theme.background)
                                .text_size(px(13.))
                                .text_color(theme.muted_foreground)
                                .child(t!("api_providers.failure_todo").to_string()),
                        )
                    });
                if menu_open {
                    body.overflow_hidden().into_any_element()
                } else {
                    body.overflow_y_scrollbar().into_any_element()
                }
            })
            .child(
                h_flex()
                    .w_full()
                    .px(px(24.))
                    .py(px(14.))
                    .border_t_1()
                    .border_color(theme.border)
                    .bg(theme.background)
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme.muted_foreground)
                            .child(t!("api_providers.save_hint").to_string()),
                    )
                    .child(
                        h_flex()
                            .gap(px(8.))
                            .child(
                                Button::new("api-ed-cancel")
                                    .label(t!("api_providers.cancel").to_string())
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.api_providers.pane = ApiProvidersPane::List;
                                        this.api_providers.draft = None;
                                        this.api_providers.kind_menu_open = false;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("api-ed-save")
                                    .custom(astrlink_primary_btn(cx))
                                    .label(save_label)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.save_api_provider_draft(window, cx);
                                    })),
                            ),
                    ),
            )
    }

    fn render_tab_divider(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        div()
            .w(px(1.))
            .h(px(14.))
            .mx(px(2.))
            .bg(theme.border.opacity(0.8))
    }

    fn render_edit_tab(
        &self,
        id: &'static str,
        tab: ApiProvidersEditTab,
        icon: IconName,
        label: String,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme().clone();
        let selected = self.api_providers.edit_tab == tab;
        h_flex()
            .id(SharedString::from(id))
            .gap(px(6.))
            .items_center()
            .px(px(12.))
            .py(px(6.))
            .rounded(px(6.))
            .cursor_pointer()
            .text_size(px(12.))
            .font_weight(FontWeight::MEDIUM)
            .when(selected, |el| {
                el.bg(theme.background)
                    .text_color(theme.foreground)
                    .shadow_sm()
            })
            .when(!selected, |el| {
                el.text_color(theme.muted_foreground)
                    .hover(|s| s.text_color(theme.foreground))
            })
            .on_click(cx.listener(move |this, _, _, cx| {
                this.api_providers.edit_tab = tab;
                cx.notify();
            }))
            .child(Icon::new(icon).size(px(13.)).text_color(if selected {
                Hsla::from(rgb(ASTRLINK_PRIMARY))
            } else {
                theme.muted_foreground
            }))
            .child(label)
    }

    fn render_kind_menu(
        &self,
        selected: ApiProviderKind,
        zh: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme().clone();
        let dark = theme.is_dark();
        let mut children: Vec<gpui::AnyElement> = Vec::new();
        for (group, kinds) in ApiProviderKind::preset_groups() {
            children.push(
                div()
                    .id(SharedString::from(format!("kind-g-{}", group_key(*group))))
                    .px(px(10.))
                    .pt(px(6.))
                    .pb(px(4.))
                    .text_size(px(11.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.muted_foreground)
                    .child(kind_group_label(*group))
                    .into_any_element(),
            );
            for k in *kinds {
                let selected_row = *k == selected;
                let kk = *k;
                let label = if zh { k.display_zh() } else { k.display_en() };
                children.push(
                    h_flex()
                        .id(SharedString::from(format!("kind-{}", k.as_str())))
                        .w_full()
                        .items_center()
                        .justify_between()
                        .gap(px(8.))
                        .px(px(10.))
                        .py(px(7.))
                        .rounded(px(6.))
                        .cursor_pointer()
                        .text_size(px(12.))
                        .when(selected_row, |el| {
                            el.bg(rgb(0xE8F1FF)).text_color(rgb(0x1D4ED8))
                        })
                        .when(!selected_row, |el| {
                            el.hover(|s| s.bg(theme.secondary.opacity(0.6)))
                        })
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.set_draft_kind(kk, window, cx);
                        }))
                        .child(
                            h_flex()
                                .items_center()
                                .gap(px(8.))
                                .min_w_0()
                                .child(brand_img(k.brand_id(), dark, px(16.)))
                                .child(div().truncate().child(label)),
                        )
                        .when(selected_row, |el| {
                            el.child(
                                Icon::new(IconName::Check)
                                    .size(px(14.))
                                    .text_color(Hsla::from(rgb(ASTRLINK_PRIMARY))),
                            )
                        })
                        .into_any_element(),
                );
            }
        }

        // Outer trap: keep wheel from chaining to the page.
        // Inner viewport: actually scrolls all 4 AstrLink groups.
        div()
            .id("api-kind-menu")
            .w_full()
            .on_scroll_wheel(|_, _, cx| {
                cx.stop_propagation();
            })
            .child(
                v_flex()
                    .id("api-kind-menu-scroll")
                    .w_full()
                    .max_h(px(360.))
                    .overflow_y_scroll()
                    .rounded(px(8.))
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.background)
                    .shadow_md()
                    .p(px(6.))
                    .gap(px(2.))
                    .children(children),
            )
    }

    fn render_models_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let models = self
            .api_providers
            .draft
            .as_ref()
            .map(|d| d.models.clone())
            .unwrap_or_default();
        let query = self.api_providers.models_query.read(cx).value().to_string();
        let filtered = filter_model_ids(&models, &query);
        let count = models.len();
        let adding = self.api_providers.models_adding;
        let bulk = self.api_providers.models_bulk;
        let fetching = self.api_providers.models_fetching;
        let show_fetch = self
            .api_providers
            .draft
            .as_ref()
            .map(|d| {
                d.kind.is_subscription()
                    || d.capabilities
                        .iter()
                        .any(|c| c.protocol == "openai.models" || c.protocol == "google.models")
            })
            .unwrap_or(false);

        v_flex()
            .w_full()
            .gap(px(12.))
            .child(
                h_flex()
                    .w_full()
                    .items_start()
                    .justify_between()
                    .gap(px(12.))
                    .child(
                        v_flex()
                            .gap(px(4.))
                            .min_w_0()
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(t!("api_providers.models_title").to_string()),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(theme.muted_foreground)
                                    .child(t!("api_providers.models_hint").to_string()),
                            ),
                    )
                    .child(
                        div()
                            .px(px(8.))
                            .py(px(2.))
                            .rounded(px(6.))
                            .bg(theme.secondary)
                            .text_size(px(11.))
                            .text_color(theme.muted_foreground)
                            .child(format!("{} {}", count, t!("api_providers.models_unit"))),
                    ),
            )
            .child(
                h_flex()
                    .w_full()
                    .gap(px(8.))
                    .items_center()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(Input::new(&self.api_providers.models_query).cleanable(true)),
                    )
                    .when(show_fetch, |el| {
                        el.child(
                            Button::new("api-models-fetch")
                                .label(if fetching {
                                    t!("api_providers.models_fetching").to_string()
                                } else {
                                    t!("api_providers.models_fetch").to_string()
                                })
                                .disabled(fetching)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.api_providers.models_fetching = false;
                                    this.api_providers.status_message =
                                        Some(t!("api_providers.models_fetch_todo").to_string());
                                    cx.notify();
                                })),
                        )
                    })
                    .child(
                        Button::new("api-models-add-toggle")
                            .icon(if adding {
                                IconName::Close
                            } else {
                                IconName::Plus
                            })
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.api_providers.models_adding =
                                    !this.api_providers.models_adding;
                                if !this.api_providers.models_adding {
                                    this.api_providers.models_bulk = false;
                                    this.api_providers.models_add_input.update(cx, |input, cx| {
                                        input.set_value("", window, cx);
                                    });
                                }
                                cx.notify();
                            })),
                    ),
            )
            .when(adding, |el| {
                el.child(
                    v_flex()
                        .w_full()
                        .gap(px(8.))
                        .p(px(12.))
                        .rounded(px(8.))
                        .border_1()
                        .border_color(theme.border)
                        .bg(theme.secondary.opacity(0.5))
                        .child(if bulk {
                            div()
                                .w_full()
                                .h(px(88.))
                                .child(Input::new(&self.api_providers.models_add_input))
                                .into_any_element()
                        } else {
                            Input::new(&self.api_providers.models_add_input)
                                .cleanable(true)
                                .into_any_element()
                        })
                        .child(
                            h_flex()
                                .w_full()
                                .justify_end()
                                .gap(px(10.))
                                .items_center()
                                .child(
                                    div()
                                        .id("api-models-bulk-toggle")
                                        .text_size(px(12.))
                                        .text_color(Hsla::from(rgb(ASTRLINK_PRIMARY)))
                                        .cursor_pointer()
                                        .child(if bulk {
                                            t!("api_providers.models_single_mode").to_string()
                                        } else {
                                            t!("api_providers.models_bulk_mode").to_string()
                                        })
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.api_providers.models_bulk =
                                                !this.api_providers.models_bulk;
                                            cx.notify();
                                        })),
                                )
                                .child(
                                    Button::new("api-models-add-submit")
                                        .label(t!("api_providers.models_add").to_string())
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.add_draft_models(window, cx);
                                        })),
                                ),
                        ),
                )
            })
            .child(if count == 0 {
                div()
                    .w_full()
                    .mt(px(4.))
                    .rounded(px(8.))
                    .border_1()
                    .border_dashed()
                    .border_color(theme.border)
                    .bg(theme.secondary.opacity(0.35))
                    .px(px(16.))
                    .py(px(28.))
                    .child(
                        div()
                            .w_full()
                            .text_center()
                            .text_size(px(12.))
                            .text_color(Hsla::from(rgb(0xB45309)))
                            .child(t!("api_providers.models_empty").to_string()),
                    )
                    .into_any_element()
            } else if filtered.is_empty() {
                div()
                    .w_full()
                    .rounded(px(8.))
                    .border_1()
                    .border_dashed()
                    .border_color(theme.border)
                    .px(px(16.))
                    .py(px(20.))
                    .child(
                        div()
                            .text_center()
                            .text_size(px(12.))
                            .text_color(theme.muted_foreground)
                            .child(t!("api_providers.models_filter_empty").to_string()),
                    )
                    .into_any_element()
            } else {
                v_flex()
                    .w_full()
                    .gap(px(6.))
                    .children(filtered.into_iter().map(|model| {
                        let mid = model.clone();
                        let mid_del = model.clone();
                        h_flex()
                            .id(SharedString::from(format!("api-model-{}", mid)))
                            .w_full()
                            .items_center()
                            .gap(px(8.))
                            .px(px(12.))
                            .py(px(10.))
                            .rounded(px(8.))
                            .border_1()
                            .border_color(theme.border)
                            .bg(theme.background)
                            .child(
                                Icon::new(IconName::Settings2)
                                    .size(px(14.))
                                    .text_color(theme.muted_foreground),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .font_family("ui-monospace, SFMono-Regular, Menlo, monospace")
                                    .text_size(px(13.))
                                    .truncate()
                                    .child(model),
                            )
                            .child(
                                Button::new(SharedString::from(format!("del-model-{}", mid_del)))
                                    .ghost()
                                    .icon(IconName::Close)
                                    .xsmall()
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        if let Some(d) = this.api_providers.draft.as_mut() {
                                            d.models.retain(|m| m != &mid_del);
                                            let joined = d.models.join("\n");
                                            this.api_providers.draft_models.update(
                                                cx,
                                                |input, cx| {
                                                    input.set_value(joined, window, cx);
                                                },
                                            );
                                        }
                                        cx.notify();
                                    })),
                            )
                    }))
                    .into_any_element()
            })
    }

    fn add_draft_models(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let raw = self
            .api_providers
            .models_add_input
            .read(cx)
            .value()
            .to_string();
        let ids = parse_model_ids(&raw);
        if ids.is_empty() {
            return;
        }
        if let Some(d) = self.api_providers.draft.as_mut() {
            for id in ids {
                if !d.models.iter().any(|m| m == &id) {
                    d.models.push(id);
                }
            }
            d.models.sort();
        }
        self.api_providers.models_add_input.update(cx, |input, cx| {
            input.set_value("", window, cx);
        });
        if !self.api_providers.models_bulk {
            self.api_providers.models_adding = false;
        }
        // Keep legacy textarea in sync for any leftover code paths.
        if let Some(d) = self.api_providers.draft.as_ref() {
            let joined = d.models.join("\n");
            self.api_providers.draft_models.update(cx, |input, cx| {
                input.set_value(joined, window, cx);
            });
        }
        cx.notify();
    }

    fn render_connection_tab(
        &self,
        creating: bool,
        is_sub: bool,
        kind: Option<ApiProviderKind>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme().clone();
        let zh = zh_locale();
        let kind = kind.unwrap_or(ApiProviderKind::CodexSubscription);
        let kind_label = if zh {
            kind.display_zh()
        } else {
            kind.display_en()
        };
        let kind_hint = if zh { kind.hint_zh() } else { kind.hint_en() };
        let enabled = self
            .api_providers
            .draft
            .as_ref()
            .map(|d| d.enabled)
            .unwrap_or(true);
        let ws = self
            .api_providers
            .draft
            .as_ref()
            .map(|d| d.responses_websocket)
            .unwrap_or(false);
        let show_ws = kind.supports_responses_websocket();
        let auth_flow = self.api_providers.auth_flow;
        let menu_open = self.api_providers.kind_menu_open;

        v_flex()
            .w_full()
            .gap(px(16.))
            .child(
                // Top: basic info | auth / HTTP
                h_flex()
                    .w_full()
                    .gap(px(16.))
                    .items_start()
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w(px(280.))
                            .rounded(px(10.))
                            .border_1()
                            .border_color(theme.border)
                            .bg(theme.background)
                            .child(
                                v_flex()
                                    .px(px(16.))
                                    .pt(px(14.))
                                    .pb(px(10.))
                                    .border_b_1()
                                    .border_color(theme.border)
                                    .gap(px(4.))
                                    .child(
                                        h_flex()
                                            .gap(px(8.))
                                            .items_center()
                                            .child(
                                                div()
                                                    .text_size(px(14.))
                                                    .text_color(Hsla::from(rgb(ASTRLINK_PRIMARY)))
                                                    .child("⚙"),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(14.))
                                                    .font_weight(FontWeight::SEMIBOLD)
                                                    .child(
                                                        t!("api_providers.basic_info").to_string(),
                                                    ),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(theme.muted_foreground)
                                            .child(
                                                t!("api_providers.basic_info_hint").to_string(),
                                            ),
                                    ),
                            )
                            .child(
                                v_flex()
                                    .p(px(16.))
                                    .gap(px(14.))
                                    // Kind selector (AstrLink groups + brand icons)
                                    .child(
                                        v_flex()
                                            .gap(px(6.))
                                            .child(
                                                div()
                                                    .text_size(px(12.))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .child(t!("api_providers.kind").to_string()),
                                            )
                                            .child(
                                                div()
                                                    .id("api-kind-trigger")
                                                    .w_full()
                                                    .px(px(12.))
                                                    .py(px(9.))
                                                    .rounded(px(8.))
                                                    .border_1()
                                                    .border_color(theme.border)
                                                    .cursor_pointer()
                                                    .when(creating, |el| {
                                                        el.on_click(cx.listener(
                                                            |this, _, _, cx| {
                                                                this.api_providers.kind_menu_open =
                                                                    !this
                                                                        .api_providers
                                                                        .kind_menu_open;
                                                                cx.notify();
                                                            },
                                                        ))
                                                    })
                                                    .when(!creating, |el| el.opacity(0.7))
                                                    .child(
                                                        h_flex()
                                                            .w_full()
                                                            .items_center()
                                                            .justify_between()
                                                            .gap(px(8.))
                                                            .child(
                                                                h_flex()
                                                                    .items_center()
                                                                    .gap(px(8.))
                                                                    .min_w_0()
                                                                    .child(brand_img(
                                                                        kind.brand_id(),
                                                                        theme.is_dark(),
                                                                        px(16.),
                                                                    ))
                                                                    .child(
                                                                        div()
                                                                            .text_size(px(13.))
                                                                            .truncate()
                                                                            .child(kind_label),
                                                                    ),
                                                            )
                                                            .when(creating, |el| {
                                                                el.child(
                                                                    Icon::new(IconName::ChevronDown)
                                                                        .size(px(14.))
                                                                        .text_color(
                                                                            theme.muted_foreground,
                                                                        ),
                                                                )
                                                            }),
                                                    ),
                                            )
                                            .when(creating && menu_open, |el| {
                                                el.child(self.render_kind_menu(kind, zh, cx))
                                            })
                                            .child(
                                                div()
                                                    .text_size(px(11.))
                                                    .text_color(theme.muted_foreground)
                                                    .child(kind_hint),
                                            ),
                                    )
                                    // Name
                                    .child(
                                        v_flex()
                                            .gap(px(6.))
                                            .child(
                                                div()
                                                    .text_size(px(12.))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .child(
                                                        t!("api_providers.service_name")
                                                            .to_string(),
                                                    ),
                                            )
                                            .child(
                                                Input::new(&self.api_providers.draft_name)
                                                    .cleanable(true),
                                            ),
                                    )
                                    // Enable
                                    .child(
                                        h_flex()
                                            .gap(px(8.))
                                            .items_center()
                                            .pt(px(4.))
                                            .border_t_1()
                                            .border_color(theme.border)
                                            .child(
                                                Checkbox::new("api-draft-enabled")
                                                    .checked(enabled)
                                                    .on_click(cx.listener(
                                                        |this, checked: &bool, _, cx| {
                                                            if let Some(d) =
                                                                this.api_providers.draft.as_mut()
                                                            {
                                                                d.enabled = *checked;
                                                            }
                                                            cx.notify();
                                                        },
                                                    )),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(12.))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .child(
                                                        t!("api_providers.enable_this").to_string(),
                                                    ),
                                            ),
                                    )
                                    .when(show_ws, |el| {
                                        el.child(
                                            h_flex()
                                                .w_full()
                                                .pt(px(8.))
                                                .border_t_1()
                                                .border_color(theme.border)
                                                .items_center()
                                                .justify_between()
                                                .child(
                                                    v_flex()
                                                        .gap(px(2.))
                                                        .child(
                                                            div()
                                                                .text_size(px(12.))
                                                                .font_weight(FontWeight::MEDIUM)
                                                                .child(
                                                                    t!("api_providers.responses_ws")
                                                                        .to_string(),
                                                                ),
                                                        )
                                                        .child(
                                                            div()
                                                                .text_size(px(11.))
                                                                .text_color(
                                                                    theme.muted_foreground,
                                                                )
                                                                .child(
                                                                    t!("api_providers.responses_ws_hint")
                                                                        .to_string(),
                                                                ),
                                                        ),
                                                )
                                                .child(
                                                    div()
                                                        .id("api-draft-ws")
                                                        .cursor_pointer()
                                                        .child(
                                                            h_flex()
                                                                .w(px(40.))
                                                                .h(px(22.))
                                                                .rounded(px(11.))
                                                                .px(px(2.))
                                                                .items_center()
                                                                .when(ws, |el| {
                                                                    el.bg(Hsla::from(rgb(
                                                                        ASTRLINK_PRIMARY,
                                                                    )))
                                                                    .justify_end()
                                                                })
                                                                .when(!ws, |el| {
                                                                    el.bg(rgb(0xD1D5DB))
                                                                        .justify_start()
                                                                })
                                                                .child(
                                                                    div()
                                                                        .size(px(18.))
                                                                        .rounded_full()
                                                                        .bg(rgb(0xFFFFFF)),
                                                                ),
                                                        )
                                                        .on_click(cx.listener(
                                                            |this, _, _, cx| {
                                                                if let Some(d) = this
                                                                    .api_providers
                                                                    .draft
                                                                    .as_mut()
                                                                {
                                                                    d.responses_websocket =
                                                                        !d.responses_websocket;
                                                                }
                                                                cx.notify();
                                                            },
                                                        )),
                                                ),
                                        )
                                    }),
                            ),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w(px(280.))
                            .rounded(px(10.))
                            .border_1()
                            .border_color(theme.border)
                            .bg(theme.background)
                            .child(
                                v_flex()
                                    .px(px(16.))
                                    .pt(px(14.))
                                    .pb(px(10.))
                                    .border_b_1()
                                    .border_color(theme.border)
                                    .gap(px(4.))
                                    .child(
                                        h_flex()
                                            .gap(px(8.))
                                            .items_center()
                                            .child(
                                                div()
                                                    .text_size(px(14.))
                                                    .text_color(Hsla::from(rgb(ASTRLINK_PRIMARY)))
                                                    .child("🔑"),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(14.))
                                                    .font_weight(FontWeight::SEMIBOLD)
                                                    .child(if is_sub {
                                                        t!("api_providers.account_auth")
                                                            .to_string()
                                                    } else {
                                                        t!("api_providers.connection_auth")
                                                            .to_string()
                                                    }),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(theme.muted_foreground)
                                            .child(if is_sub {
                                                t!("api_providers.account_auth_hint").to_string()
                                            } else {
                                                t!("api_providers.connection_auth_hint")
                                                    .to_string()
                                            }),
                                    ),
                            )
                            .child(
                                v_flex()
                                    .p(px(16.))
                                    .gap(px(12.))
                                    .when(is_sub, |el| {
                                        el.when(
                                            kind == ApiProviderKind::ClaudeSubscription,
                                            |el| {
                                                el.child(self.render_auth_choice(
                                                    ApiAuthFlow::AuthorizationCode,
                                                    t!("api_providers.claude_oauth").to_string(),
                                                    t!("api_providers.claude_oauth_hint")
                                                        .to_string(),
                                                    auth_flow
                                                        == ApiAuthFlow::AuthorizationCode,
                                                    false,
                                                    cx,
                                                ))
                                            },
                                        )
                                        .when(
                                            kind == ApiProviderKind::GrokSubscription,
                                            |el| {
                                                el.child(self.render_auth_choice(
                                                    ApiAuthFlow::DeviceCode,
                                                    "Device Code".into(),
                                                    t!("api_providers.grok_device_hint")
                                                        .to_string(),
                                                    true,
                                                    false,
                                                    cx,
                                                ))
                                            },
                                        )
                                        .when(
                                            kind == ApiProviderKind::CodexSubscription,
                                            |el| {
                                                el.child(self.render_auth_choice(
                                                    ApiAuthFlow::Browser,
                                                    t!("api_providers.browser_oauth").to_string(),
                                                    t!("api_providers.browser_oauth_hint")
                                                        .to_string(),
                                                    auth_flow == ApiAuthFlow::Browser,
                                                    true,
                                                    cx,
                                                ))
                                                .child(self.render_auth_choice(
                                                    ApiAuthFlow::DeviceCode,
                                                    "Device Code".into(),
                                                    t!("api_providers.device_code_hint")
                                                        .to_string(),
                                                    auth_flow == ApiAuthFlow::DeviceCode,
                                                    true,
                                                    cx,
                                                ))
                                            },
                                        )
                                        .child(
                                            h_flex()
                                                .gap(px(8.))
                                                .items_start()
                                                .mt(px(4.))
                                                .p(px(12.))
                                                .rounded(px(8.))
                                                .border_1()
                                                .border_color(rgb(0xBBF7D0))
                                                .bg(rgb(0xF0FDF4))
                                                .child(
                                                    div()
                                                        .mt(px(2.))
                                                        .size(px(8.))
                                                        .rounded_full()
                                                        .bg(rgb(0x16A34A)),
                                                )
                                                .child(
                                                    v_flex()
                                                        .gap(px(2.))
                                                        .child(
                                                            div()
                                                                .text_size(px(13.))
                                                                .font_weight(FontWeight::MEDIUM)
                                                                .text_color(rgb(0x166534))
                                                                .child(
                                                                    t!("api_providers.save_then_login")
                                                                        .to_string(),
                                                                ),
                                                        )
                                                        .child(
                                                            div()
                                                                .text_size(px(11.))
                                                                .text_color(rgb(0x15803D))
                                                                .child(
                                                                    t!("api_providers.account_auth_hint")
                                                                        .to_string(),
                                                                ),
                                                        ),
                                                ),
                                        )
                                    })
                                    .when(!is_sub, |el| {
                                        el.child(
                                            v_flex()
                                                .gap(px(6.))
                                                .child(
                                                    div()
                                                        .text_size(px(12.))
                                                        .font_weight(FontWeight::MEDIUM)
                                                        .child(
                                                            t!("api_providers.base_url")
                                                                .to_string(),
                                                        ),
                                                )
                                                .child(
                                                    Input::new(
                                                        &self.api_providers.draft_base_url,
                                                    )
                                                    .cleanable(true),
                                                ),
                                        )
                                        .child(
                                            v_flex()
                                                .gap(px(6.))
                                                .child(
                                                    div()
                                                        .text_size(px(12.))
                                                        .font_weight(FontWeight::MEDIUM)
                                                        .child(
                                                            t!("api_providers.api_key").to_string(),
                                                        ),
                                                )
                                                .child(
                                                    Input::new(
                                                        &self.api_providers.draft_api_key,
                                                    )
                                                    .cleanable(true),
                                                ),
                                        )
                                    }),
                            ),
                    ),
            )
            .child(
                // Proxy panel
                v_flex()
                    .w_full()
                    .rounded(px(10.))
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.background)
                    .child(
                        v_flex()
                            .px(px(16.))
                            .pt(px(14.))
                            .pb(px(10.))
                            .border_b_1()
                            .border_color(theme.border)
                            .gap(px(4.))
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(t!("api_providers.proxy_title").to_string()),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(theme.muted_foreground)
                                    .child(t!("api_providers.proxy_hint").to_string()),
                            ),
                    )
                    .child(
                        v_flex()
                            .p(px(16.))
                            .gap(px(6.))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(t!("api_providers.proxy_mode").to_string()),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .max_w(px(320.))
                                    .px(px(12.))
                                    .py(px(9.))
                                    .rounded(px(8.))
                                    .border_1()
                                    .border_color(theme.border)
                                    .text_size(px(13.))
                                    .child(t!("api_providers.proxy_inherit").to_string()),
                            ),
                    ),
            )
    }

    fn render_auth_choice(
        &self,
        flow: ApiAuthFlow,
        label: String,
        hint: String,
        selected: bool,
        selectable: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme().clone();
        div()
            .id(SharedString::from(format!("auth-flow-{:?}", flow)))
            .w_full()
            .px(px(12.))
            .py(px(10.))
            .rounded(px(8.))
            .border_1()
            .border_color(if selected {
                Hsla::from(rgb(ASTRLINK_PRIMARY))
            } else {
                theme.border
            })
            .bg(if selected {
                Hsla::from(rgb(0xF0F5FA))
            } else {
                theme.background
            })
            .when(selectable, |el| {
                el.cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.api_providers.auth_flow = flow;
                        cx.notify();
                    }))
            })
            .child(
                h_flex()
                    .gap(px(10.))
                    .items_start()
                    .child(
                        div()
                            .mt(px(2.))
                            .size(px(14.))
                            .rounded_full()
                            .border_1()
                            .border_color(if selected {
                                Hsla::from(rgb(ASTRLINK_PRIMARY))
                            } else {
                                theme.border
                            })
                            .flex()
                            .items_center()
                            .justify_center()
                            .when(selected, |el| {
                                el.child(
                                    div()
                                        .size(px(8.))
                                        .rounded_full()
                                        .bg(Hsla::from(rgb(ASTRLINK_PRIMARY))),
                                )
                            }),
                    )
                    .child(
                        v_flex()
                            .gap(px(2.))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(label),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(theme.muted_foreground)
                                    .child(hint),
                            ),
                    ),
            )
    }

    fn render_protocols_tab(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let caps = self
            .api_providers
            .draft
            .as_ref()
            .map(|d| d.capabilities.clone())
            .unwrap_or_default();
        v_flex()
            .gap(px(8.))
            .child(
                div()
                    .text_size(px(13.))
                    .text_color(theme.muted_foreground)
                    .child(t!("api_providers.protocols_hint").to_string()),
            )
            .children(caps.into_iter().map(|c| {
                h_flex()
                    .gap(px(8.))
                    .px(px(10.))
                    .py(px(8.))
                    .rounded(px(8.))
                    .bg(theme.secondary.opacity(0.4))
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .child(c.protocol),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(theme.muted_foreground)
                            .child(c.mode),
                    )
                    .when(c.streaming, |el| {
                        el.child(
                            div()
                                .text_size(px(11.))
                                .text_color(rgb(0x2563EB))
                                .child("streaming"),
                        )
                    })
            }))
    }
}
