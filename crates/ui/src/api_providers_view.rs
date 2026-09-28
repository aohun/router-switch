//! AstrLink-aligned API Providers page (gateway upstream registry).
//! Distinct from per-app Live `Route::Codex` / Claude / … provider pages.

use domain::{
    merge_visible_api_provider_order, ApiCapability, ApiProvider, ApiProviderKind,
    ApiProviderKindGroup, HttpConnection, SubscriptionStatus,
};
use gpui::{
    div, prelude::FluentBuilder, px, relative, rgb, AnyElement, App, AppContext, Context, Corner,
    Entity, FontWeight, Hsla, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, Window,
};
use gpui_component::{
    button::{Button, ButtonCustomVariant, ButtonVariants},
    checkbox::Checkbox,
    h_flex,
    input::{Input, InputState},
    menu::{DropdownMenu, PopupMenuItem},
    notification::Notification,
    scroll::ScrollableElement as _,
    v_flex, ActiveTheme, Disableable, Icon, IconName, Sizable, WindowExt,
};
use rust_i18n::t;

use crate::app_view::RouterApp;
use crate::assets::{brand_img, CustomIcon};

/// Drag payload for API provider list reorder (AstrLink OrderedList).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DragApiProviderId(pub String);

/// Lifted-row ghost while dragging (AstrLink OrderedList: shadow-lg + ring-primary/60).
pub struct ApiProviderDragGhost {
    pub name: SharedString,
    pub kind_label: SharedString,
}

impl gpui::Render for ApiProviderDragGhost {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        h_flex()
            .items_center()
            .gap(px(10.))
            .px(px(14.))
            .py(px(12.))
            .min_w(px(280.))
            .max_w(px(480.))
            .rounded(px(8.))
            .bg(theme.background)
            // AstrLink: `ring-1 ring-inset ring-primary/60` + `shadow-lg`
            .border_1()
            .border_color(astrlink_primary(0.6))
            .shadow_lg()
            .child(
                div()
                    .size(px(28.))
                    .rounded(px(6.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(Hsla::from(rgb(ASTRLINK_PRIMARY)))
                    .child(Icon::new(CustomIcon::GripVertical).size(px(16.))),
            )
            .child(
                v_flex()
                    .gap(px(2.))
                    .min_w(px(0.))
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.foreground)
                            .child(self.name.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(theme.muted_foreground)
                            .child(self.kind_label.clone()),
                    ),
            )
    }
}

/// AstrLink light-mode `--primary-fill` / `--primary-hover`.
const ASTRLINK_PRIMARY: u32 = 0x1D4D87;
const ASTRLINK_PRIMARY_HOVER: u32 = 0x16345C;
const ASTRLINK_PRIMARY_ACTIVE: u32 = 0x122B4C;

fn astrlink_primary(alpha: f32) -> Hsla {
    Hsla::from(rgb(ASTRLINK_PRIMARY)).opacity(alpha)
}

pub(crate) fn astrlink_primary_btn(cx: &App) -> ButtonCustomVariant {
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
    /// Codex: user must pick Browser or Device Code explicitly.
    #[default]
    Unset,
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
    /// API provider id awaiting OAuth completion (device-code path).
    pub pending_oauth_provider_id: Option<String>,
    /// Codex subscription usage snapshots keyed by api provider id.
    pub usage_by_id: std::collections::HashMap<String, session::CodexSubscriptionUsage>,
    pub usage_error_by_id: std::collections::HashMap<String, String>,
    pub usage_loading: std::collections::HashSet<String>,
    pub status_message: Option<String>,
    /// Persisting list order after drag / keyboard nudge.
    pub order_saving: bool,
    /// Row currently being dragged (source placeholder / grip ring).
    pub dragging_id: Option<String>,
    /// Row under the pointer while dragging (AstrLink drop-slot accent).
    pub drop_hover_id: Option<String>,
    /// Briefly highlighted after a successful drop (AstrLink settle).
    pub settled_id: Option<String>,
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
            auth_flow: ApiAuthFlow::Unset,
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
            pending_oauth_provider_id: None,
            usage_by_id: std::collections::HashMap::new(),
            usage_error_by_id: std::collections::HashMap::new(),
            usage_loading: std::collections::HashSet::new(),
            status_message: None,
            order_saving: false,
            dragging_id: None,
            drop_hover_id: None,
            settled_id: None,
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
        self.refresh_connected_codex_usage(cx);
        cx.notify();
    }

    /// Pull `/wham/usage` for connected Codex subscription providers (AstrLink list meter).
    pub(crate) fn refresh_connected_codex_usage(&mut self, cx: &mut Context<Self>) {
        let ids: Vec<String> = self
            .api_providers
            .items
            .iter()
            .filter(|p| {
                p.kind == ApiProviderKind::CodexSubscription
                    && p.subscription.as_ref().is_some_and(|s| {
                        matches!(
                            s.status,
                            SubscriptionStatus::Connected | SubscriptionStatus::NeedsReauth
                        )
                    })
            })
            .map(|p| p.id.clone())
            .collect();
        for id in ids {
            self.fetch_codex_usage_for_provider(&id, cx);
        }
    }

    fn fetch_codex_usage_for_provider(&mut self, provider_id: &str, cx: &mut Context<Self>) {
        if !self
            .api_providers
            .usage_loading
            .insert(provider_id.to_string())
        {
            return;
        }
        let (db_path, codex_paths) = self.workspace.codex_subscription_fetch_context();
        cx.notify();
        let id = provider_id.to_string();
        cx.spawn(
            move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
                let mut cx = cx.clone();
                async move {
                    let usage = session::tokio_runtime()
                        .spawn_blocking(move || {
                            let store = store::Store::open(&db_path)
                                .map_err(|e| format!("打开数据库失败: {e}"))?;
                            session::tokio_runtime().handle().block_on(
                                session::fetch_codex_usage_with_refresh(&store, &codex_paths),
                            )
                        })
                        .await
                        .unwrap_or_else(|e| Err(format!("任务失败: {e}")));
                    let _ = this.update(&mut cx, |this, cx| {
                        this.api_providers.usage_loading.remove(&id);
                        match usage {
                            Ok(usage) => {
                                this.api_providers.usage_error_by_id.remove(&id);
                                if let Some(p) =
                                    this.api_providers.items.iter_mut().find(|p| p.id == id)
                                {
                                    let mut dirty = false;
                                    if let Some(sub) = p.subscription.as_mut() {
                                        if sub.status != SubscriptionStatus::Connected
                                            || sub.last_error.is_some()
                                        {
                                            sub.status = SubscriptionStatus::Connected;
                                            sub.last_error = None;
                                            dirty = true;
                                        }
                                        if let Ok(Some(tokens)) =
                                            this.workspace.codex_subscription_tokens()
                                        {
                                            if let Some(account_id) = tokens.account_id.as_deref() {
                                                let hint = session::mask_account_hint(account_id);
                                                if !hint.is_empty()
                                                    && sub.account_hint.as_deref()
                                                        != Some(hint.as_str())
                                                {
                                                    sub.account_hint = Some(hint);
                                                    dirty = true;
                                                }
                                            }
                                        }
                                    }
                                    if dirty {
                                        p.updated_at = now_ms();
                                        let _ = this.workspace.upsert_api_provider(p);
                                    }
                                }
                                this.api_providers.usage_by_id.insert(id, usage);
                            }
                            Err(err) => {
                                this.api_providers.usage_by_id.remove(&id);
                                let needs_reauth =
                                    session::codex_subscription::is_token_auth_error(&err)
                                        || err.contains("请重新授权");
                                let msg = if needs_reauth {
                                    t!("api_providers.needs_reauth").to_string()
                                } else {
                                    err
                                };
                                if needs_reauth {
                                    if let Some(p) =
                                        this.api_providers.items.iter_mut().find(|p| p.id == id)
                                    {
                                        if let Some(sub) = p.subscription.as_mut() {
                                            sub.status = SubscriptionStatus::NeedsReauth;
                                            sub.last_error = Some(msg.clone());
                                        }
                                        p.updated_at = now_ms();
                                        let _ = this.workspace.upsert_api_provider(p);
                                    }
                                }
                                this.api_providers.usage_error_by_id.insert(id, msg);
                            }
                        }
                        cx.notify();
                    });
                }
            },
        )
        .detach();
    }

    /// After OAuth: merge discovered models + refresh usage (AstrLink importCodexModelsAfterLogin).
    pub(crate) fn sync_codex_after_oauth(
        &mut self,
        provider_id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (db_path, codex_paths) = self.workspace.codex_subscription_fetch_context();
        let id = provider_id.to_string();
        let view = cx.entity().downgrade();
        window
            .spawn(cx, move |cx: &mut gpui::AsyncWindowContext| {
                let mut cx = cx.clone();
                async move {
                    // Single ensure → models → usage (never parallel-refresh the one-time RT).
                    let result = session::tokio_runtime()
                        .spawn_blocking(move || {
                            let store = store::Store::open(&db_path)
                                .map_err(|e| format!("打开数据库失败: {e}"))?;
                            session::tokio_runtime().handle().block_on(
                                session::fetch_codex_models_and_usage(&store, &codex_paths),
                            )
                        })
                        .await
                        .unwrap_or_else(|e| Err(format!("任务失败: {e}")));

                    let _ = cx.update(|window: &mut Window, cx: &mut App| {
                        let _ = view.update(cx, |this, cx| match result {
                            Ok((discovered, usage)) => {
                                if let Some(p) =
                                    this.api_providers.items.iter_mut().find(|p| p.id == id)
                                {
                                    let mut merged: Vec<String> = p
                                        .models
                                        .iter()
                                        .cloned()
                                        .chain(discovered.iter().cloned())
                                        .collect();
                                    merged.sort();
                                    merged.dedup();
                                    let count = merged.len();
                                    if merged != p.models {
                                        p.models = merged;
                                        p.updated_at = now_ms();
                                        let _ = this.workspace.upsert_api_provider(p);
                                    }
                                    this.api_providers.status_message = Some(if count == 0 {
                                        t!("api_providers.models_fetched_none").to_string()
                                    } else {
                                        t!("api_providers.models_fetched", count = count)
                                            .to_string()
                                    });
                                    window.push_notification(
                                        Notification::success(
                                            t!("api_providers.models_fetched", count = count)
                                                .to_string(),
                                        ),
                                        cx,
                                    );
                                }
                                this.api_providers.usage_error_by_id.remove(&id);
                                this.api_providers.usage_by_id.insert(id.clone(), usage);
                                this.refresh_api_providers(cx);
                            }
                            Err(err) => {
                                this.api_providers
                                    .usage_error_by_id
                                    .insert(id.clone(), err.clone());
                                window.push_notification(
                                    Notification::warning(format!(
                                        "{} ({err})",
                                        t!("api_providers.models_fetch_failed")
                                    )),
                                    cx,
                                );
                                cx.notify();
                            }
                        });
                    });
                }
            })
            .detach();
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
        self.api_providers.auth_flow = match p.kind {
            ApiProviderKind::CodexSubscription => ApiAuthFlow::Browser,
            ApiProviderKind::GrokSubscription => ApiAuthFlow::DeviceCode,
            ApiProviderKind::ClaudeSubscription => ApiAuthFlow::AuthorizationCode,
            _ => ApiAuthFlow::Unset,
        };
        let mut draft = p.clone();
        draft
            .kind
            .ensure_subscription_natives(&mut draft.capabilities);
        self.api_providers.draft = Some(draft);
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
        let creating = matches!(self.api_providers.pane, ApiProvidersPane::Create);
        let name = self.api_providers.draft_name.read(cx).value().to_string();
        let name = name.trim().to_string();
        if name.is_empty() {
            self.api_providers.status_message =
                Some(t!("api_providers.err_name_required").to_string());
            cx.notify();
            return;
        }
        // AstrLink: create/re-auth subscription requires an explicit login transport.
        let needs_reauth = draft.subscription.as_ref().is_some_and(|s| {
            matches!(
                s.status,
                SubscriptionStatus::NeedsReauth
                    | SubscriptionStatus::Disconnected
                    | SubscriptionStatus::Error
            )
        });
        if (creating || needs_reauth)
            && draft.kind == ApiProviderKind::CodexSubscription
            && matches!(self.api_providers.auth_flow, ApiAuthFlow::Unset)
        {
            let msg = t!("api_providers.err_choose_login").to_string();
            self.api_providers.status_message = Some(msg.clone());
            window.push_notification(Notification::warning(msg), cx);
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
        draft.updated_at = now_ms();
        draft
            .kind
            .ensure_subscription_natives(&mut draft.capabilities);
        let auth_flow = self.api_providers.auth_flow;
        let provider_id = draft.id.clone();
        let kind = draft.kind;
        let should_login = kind.is_subscription() && (creating || needs_reauth);
        if should_login {
            if let Some(sub) = draft.subscription.as_mut() {
                sub.status = SubscriptionStatus::Authorizing;
                sub.last_error = None;
            }
        }

        match self.workspace.upsert_api_provider(&draft) {
            Ok(()) => {
                // Auto-probe only on create (or empty allow-list). Protocol-only edits →「已保存」.
                let auto_probe = kind.supports_http_model_probe()
                    && (creating || draft.models.is_empty())
                    && draft
                        .http
                        .as_ref()
                        .is_some_and(|h| !h.base_url.trim().is_empty());
                let probe_http = draft.http.clone();
                self.api_providers.pane = ApiProvidersPane::List;
                self.api_providers.draft = None;
                self.api_providers.kind_menu_open = false;
                self.refresh_api_providers(cx);
                if should_login {
                    self.begin_api_provider_authorization(
                        &provider_id,
                        kind,
                        auth_flow,
                        window,
                        cx,
                    );
                } else if auto_probe {
                    if let Some(http) = probe_http {
                        self.api_providers.status_message =
                            Some(t!("api_providers.models_fetching").to_string());
                        cx.notify();
                        self.probe_and_persist_api_provider_models(
                            &provider_id,
                            http,
                            true,
                            window,
                            cx,
                        );
                    }
                } else {
                    let msg = t!("api_providers.saved").to_string();
                    self.api_providers.status_message = Some(msg.clone());
                    window.push_notification(Notification::success(msg), cx);
                    cx.notify();
                }
            }
            Err(err) => {
                self.api_providers.status_message = Some(err.to_string());
                cx.notify();
            }
        }
    }

    /// Copy Connection-tab inputs into `draft.http` before probing models.
    fn sync_draft_http_from_inputs(&mut self, cx: &mut Context<Self>) {
        let Some(draft) = self.api_providers.draft.as_mut() else {
            return;
        };
        if draft.kind.is_subscription() {
            return;
        }
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
        let mut http = draft.http.clone().unwrap_or_else(|| HttpConnection {
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

    /// Models-tab「获取模型列表」(AstrLink discoverModels → merge into draft).
    pub(crate) fn fetch_draft_api_provider_models(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.api_providers.models_fetching {
            return;
        }
        self.sync_draft_http_from_inputs(cx);
        let Some(draft) = self.api_providers.draft.as_ref() else {
            return;
        };
        if draft.kind.is_subscription() {
            // Subscriptions use OAuth probe paths; HTTP /v1/models needs a saved credential.
            self.api_providers.status_message =
                Some(t!("api_providers.models_fetch_need_save").to_string());
            window.push_notification(
                Notification::warning(t!("api_providers.models_fetch_need_save").to_string()),
                cx,
            );
            cx.notify();
            return;
        }
        let Some(http) = draft.http.clone() else {
            self.api_providers.status_message =
                Some(t!("api_providers.models_fetch_need_base").to_string());
            window.push_notification(
                Notification::warning(t!("api_providers.models_fetch_need_base").to_string()),
                cx,
            );
            cx.notify();
            return;
        };
        if http.base_url.trim().is_empty() {
            self.api_providers.status_message =
                Some(t!("api_providers.models_fetch_need_base").to_string());
            window.push_notification(
                Notification::warning(t!("api_providers.models_fetch_need_base").to_string()),
                cx,
            );
            cx.notify();
            return;
        }

        self.api_providers.models_fetching = true;
        self.api_providers.status_message = Some(t!("api_providers.models_fetching").to_string());
        cx.notify();

        let view = cx.entity().downgrade();
        window
            .spawn(cx, move |cx: &mut gpui::AsyncWindowContext| {
                let mut cx = cx.clone();
                async move {
                    let result = cx
                        .background_executor()
                        .spawn(async move { domain::fetch_http_connection_models(&http) })
                        .await;
                    let _ = cx.update(|window: &mut Window, cx: &mut App| {
                        let _ = view.update(cx, |this, cx| {
                            this.api_providers.models_fetching = false;
                            match result {
                                Ok(discovered) => {
                                    let Some(draft) = this.api_providers.draft.as_mut() else {
                                        cx.notify();
                                        return;
                                    };
                                    let current = draft.models.clone();
                                    match domain::merge_discovered_api_models(&current, &discovered)
                                    {
                                        Some(merged) => {
                                            let count = merged.len();
                                            let added = merged
                                                .iter()
                                                .filter(|m| !current.contains(m))
                                                .count();
                                            draft.models = merged;
                                            let joined = draft.models.join("\n");
                                            this.api_providers.draft_models.update(
                                                cx,
                                                |input, cx| {
                                                    input.set_value(joined, window, cx);
                                                },
                                            );
                                            let msg = if count == 0 {
                                                t!("api_providers.models_fetch_empty").to_string()
                                            } else {
                                                t!(
                                                    "api_providers.models_fetch_ok",
                                                    count = count,
                                                    added = added
                                                )
                                                .to_string()
                                            };
                                            this.api_providers.status_message = Some(msg.clone());
                                            window.push_notification(
                                                if count == 0 {
                                                    Notification::warning(msg)
                                                } else {
                                                    Notification::success(msg)
                                                },
                                                cx,
                                            );
                                        }
                                        None => {
                                            let msg = t!("api_providers.models_fetch_too_many")
                                                .to_string();
                                            this.api_providers.status_message = Some(msg.clone());
                                            window
                                                .push_notification(Notification::warning(msg), cx);
                                        }
                                    }
                                    cx.notify();
                                }
                                Err(err) => {
                                    let msg = t!("api_providers.models_fetch_error", error = err)
                                        .to_string();
                                    this.api_providers.status_message = Some(msg.clone());
                                    window.push_notification(Notification::warning(msg), cx);
                                    cx.notify();
                                }
                            }
                        });
                    });
                }
            })
            .detach();
    }

    /// After create/save: probe upstream and persist merged models (AstrLink subscription import style).
    pub(crate) fn probe_and_persist_api_provider_models(
        &mut self,
        provider_id: &str,
        http: HttpConnection,
        show_saved_toast: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let id = provider_id.to_string();
        let view = cx.entity().downgrade();
        window
            .spawn(cx, move |cx: &mut gpui::AsyncWindowContext| {
                let mut cx = cx.clone();
                async move {
                    let result = cx
                        .background_executor()
                        .spawn(async move { domain::fetch_http_connection_models(&http) })
                        .await;
                    let _ = cx.update(|window: &mut Window, cx: &mut App| {
                        let _ = view.update(cx, |this, cx| match result {
                            Ok(discovered) => {
                                let Some(idx) =
                                    this.api_providers.items.iter().position(|p| p.id == id)
                                else {
                                    return;
                                };
                                let current = this.api_providers.items[idx].models.clone();
                                match domain::merge_discovered_api_models(&current, &discovered) {
                                    Some(merged) if merged != current => {
                                        let added =
                                            merged.iter().filter(|m| !current.contains(m)).count();
                                        this.api_providers.items[idx].models = merged.clone();
                                        this.api_providers.items[idx].updated_at = now_ms();
                                        let provider = this.api_providers.items[idx].clone();
                                        if let Err(err) =
                                            this.workspace.upsert_api_provider(&provider)
                                        {
                                            this.api_providers.status_message =
                                                Some(err.to_string());
                                            cx.notify();
                                            return;
                                        }
                                        let count = merged.len();
                                        let sync = t!(
                                            "api_providers.models_fetch_ok",
                                            count = count,
                                            added = added
                                        )
                                        .to_string();
                                        let msg = if show_saved_toast {
                                            format!("{} · {sync}", t!("api_providers.saved"))
                                        } else {
                                            sync
                                        };
                                        this.api_providers.status_message = Some(msg.clone());
                                        window.push_notification(Notification::success(msg), cx);
                                        this.refresh_api_providers(cx);
                                    }
                                    Some(merged) => {
                                        // No model changes — don't pretend we "synced models".
                                        let msg = if show_saved_toast {
                                            t!("api_providers.saved").to_string()
                                        } else if merged.is_empty() {
                                            t!("api_providers.models_fetch_empty").to_string()
                                        } else {
                                            t!(
                                                "api_providers.models_fetch_ok",
                                                count = merged.len(),
                                                added = 0
                                            )
                                            .to_string()
                                        };
                                        this.api_providers.status_message = Some(msg.clone());
                                        window.push_notification(
                                            if !show_saved_toast && merged.is_empty() {
                                                Notification::warning(msg)
                                            } else {
                                                Notification::success(msg)
                                            },
                                            cx,
                                        );
                                        cx.notify();
                                    }
                                    None => {
                                        let msg =
                                            t!("api_providers.models_fetch_too_many").to_string();
                                        this.api_providers.status_message = Some(msg.clone());
                                        window.push_notification(Notification::warning(msg), cx);
                                        cx.notify();
                                    }
                                }
                            }
                            Err(err) => {
                                let msg = if show_saved_toast {
                                    format!(
                                        "{} · {}",
                                        t!("api_providers.saved"),
                                        t!("api_providers.models_fetch_error", error = err)
                                    )
                                } else {
                                    t!("api_providers.models_fetch_error", error = err).to_string()
                                };
                                this.api_providers.status_message = Some(msg.clone());
                                window.push_notification(Notification::warning(msg), cx);
                                cx.notify();
                            }
                        });
                    });
                }
            })
            .detach();
    }

    /// AstrLink-style authorize after create (or list re-login).
    pub(crate) fn begin_api_provider_authorization(
        &mut self,
        provider_id: &str,
        kind: ApiProviderKind,
        flow: ApiAuthFlow,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match kind {
            ApiProviderKind::CodexSubscription => match flow {
                ApiAuthFlow::Browser => {
                    self.start_codex_browser_for_api_provider(provider_id, window, cx);
                }
                ApiAuthFlow::DeviceCode => {
                    self.api_providers.pending_oauth_provider_id = Some(provider_id.to_string());
                    self.start_oauth_login(session::CODEX_PROVIDER, window, cx);
                }
                ApiAuthFlow::Unset | ApiAuthFlow::AuthorizationCode => {
                    let msg = t!("api_providers.err_choose_login").to_string();
                    self.mark_api_provider_auth_error(provider_id, msg, cx);
                }
            },
            ApiProviderKind::GrokSubscription => {
                self.api_providers.pending_oauth_provider_id = Some(provider_id.to_string());
                self.start_oauth_login(session::XAI_PROVIDER, window, cx);
            }
            ApiProviderKind::ClaudeSubscription => {
                let msg = t!("api_providers.claude_oauth_todo").to_string();
                self.mark_api_provider_auth_error(provider_id, msg.clone(), cx);
                window.push_notification(
                    Notification::info(t!("api_providers.added_later_login").to_string()),
                    cx,
                );
            }
            _ => {}
        }
    }

    pub(crate) fn mark_api_provider_auth_error(
        &mut self,
        provider_id: &str,
        err: String,
        cx: &mut Context<Self>,
    ) {
        if let Some(p) = self
            .api_providers
            .items
            .iter_mut()
            .find(|p| p.id == provider_id)
        {
            if let Some(sub) = p.subscription.as_mut() {
                sub.status = SubscriptionStatus::Error;
                sub.last_error = Some(err.clone());
            }
            p.updated_at = now_ms();
            let _ = self.workspace.upsert_api_provider(p);
        }
        self.api_providers.status_message = Some(err);
        cx.notify();
    }

    pub(crate) fn mark_api_provider_connected(
        &mut self,
        provider_id: &str,
        account_hint: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let kind = self
            .api_providers
            .items
            .iter()
            .find(|p| p.id == provider_id)
            .map(|p| p.kind);
        if let Some(p) = self
            .api_providers
            .items
            .iter_mut()
            .find(|p| p.id == provider_id)
        {
            if let Some(sub) = p.subscription.as_mut() {
                sub.status = SubscriptionStatus::Connected;
                sub.account_hint = account_hint;
                sub.last_error = None;
            }
            p.updated_at = now_ms();
            let _ = self.workspace.upsert_api_provider(p);
        }
        self.refresh_api_providers(cx);
        if matches!(kind, Some(ApiProviderKind::CodexSubscription)) {
            self.sync_codex_after_oauth(provider_id, window, cx);
        }
    }

    pub(crate) fn account_hint_from_tokens(tokens: &session::AuthTokens) -> Option<String> {
        if let Some(account_id) = tokens
            .account_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            let masked = session::mask_account_hint(account_id);
            if !masked.is_empty() {
                return Some(masked);
            }
        }
        tokens
            .email
            .clone()
            .filter(|s| !s.trim().is_empty())
            .map(|s| {
                if s.len() > 4 {
                    format!("…{}", &s[s.len().saturating_sub(4)..])
                } else {
                    s
                }
            })
    }

    /// Codex browser OAuth with loopback callback (AstrLink browser flow).
    fn start_codex_browser_for_api_provider(
        &mut self,
        provider_id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let started = self.workspace.oauth_start_codex_browser();
        let (auth_url, rx) = match started {
            Ok(pair) => pair,
            Err(err) => {
                let msg = err.to_string();
                // Ports busy → Device Code fallback (AstrLink).
                if msg.contains("1455") || msg.contains("1457") || msg.contains("不可用") {
                    window.push_notification(
                        Notification::info(t!("api_providers.browser_fallback_device").to_string()),
                        cx,
                    );
                    self.api_providers.auth_flow = ApiAuthFlow::DeviceCode;
                    self.api_providers.pending_oauth_provider_id = Some(provider_id.to_string());
                    self.start_oauth_login(session::CODEX_PROVIDER, window, cx);
                    return;
                }
                self.mark_api_provider_auth_error(provider_id, msg.clone(), cx);
                window.push_notification(
                    Notification::warning(format!(
                        "{} ({msg})",
                        t!("api_providers.added_later_login")
                    )),
                    cx,
                );
                return;
            }
        };

        cx.open_url(&auth_url);
        window.push_notification(
            Notification::info(t!("api_providers.oauth_browser_opened").to_string()),
            cx,
        );
        self.api_providers.status_message =
            Some(t!("api_providers.oauth_browser_opened").to_string());
        cx.notify();

        let provider_id = provider_id.to_string();
        let view = cx.entity().downgrade();
        window
            .spawn(cx, move |cx: &mut gpui::AsyncWindowContext| {
                let mut cx = cx.clone();
                async move {
                    let result = cx
                        .background_executor()
                        .spawn(async move {
                            let deadline =
                                std::time::Instant::now() + std::time::Duration::from_secs(10 * 60);
                            loop {
                                match rx.try_recv() {
                                    Ok(v) => return v,
                                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                                        if std::time::Instant::now() >= deadline {
                                            return Err("等待 OAuth 回调超时".into());
                                        }
                                        std::thread::sleep(std::time::Duration::from_millis(200));
                                    }
                                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                                        return Err("OAuth 回调通道已断开".into());
                                    }
                                }
                            }
                        })
                        .await;

                    let _ = cx.update(|window: &mut Window, cx: &mut App| {
                        let _ = view.update(cx, |this, cx| match result {
                            Ok(tokens) => {
                                match this
                                    .workspace
                                    .oauth_complete(session::CODEX_PROVIDER, &tokens)
                                {
                                    Ok(()) => {
                                        this.reload_oauth_statuses();
                                        let hint = Self::account_hint_from_tokens(&tokens);
                                        this.mark_api_provider_connected(
                                            &provider_id,
                                            hint,
                                            window,
                                            cx,
                                        );
                                        this.api_providers.status_message =
                                            Some(t!("auth.login_success").to_string());
                                        window.push_notification(
                                            Notification::success(
                                                t!("auth.login_success").to_string(),
                                            ),
                                            cx,
                                        );
                                    }
                                    Err(err) => {
                                        this.mark_api_provider_auth_error(
                                            &provider_id,
                                            err.to_string(),
                                            cx,
                                        );
                                        window.push_notification(
                                            Notification::error(err.to_string()),
                                            cx,
                                        );
                                    }
                                }
                            }
                            Err(err) => {
                                this.mark_api_provider_auth_error(&provider_id, err.clone(), cx);
                                window.push_notification(
                                    Notification::warning(format!(
                                        "{} ({err})",
                                        t!("api_providers.added_later_login")
                                    )),
                                    cx,
                                );
                            }
                        });
                    });
                }
            })
            .detach();
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

    /// Persist a new global order. `visible_ordered_ids` may be a filtered subset
    /// (AstrLink: only replace those slots in the full priority list).
    /// `highlight_id` is briefly ring-highlighted after a successful drop.
    pub(crate) fn apply_api_provider_reorder(
        &mut self,
        visible_ordered_ids: &[String],
        highlight_id: Option<&str>,
        cx: &mut Context<Self>,
    ) {
        if self.api_providers.order_saving || visible_ordered_ids.len() < 2 {
            return;
        }
        let global_ids: Vec<String> = self
            .api_providers
            .items
            .iter()
            .map(|p| p.id.clone())
            .collect();
        let Some(merged) = merge_visible_api_provider_order(&global_ids, visible_ordered_ids)
        else {
            return;
        };
        if merged == global_ids {
            self.api_providers.dragging_id = None;
            self.api_providers.drop_hover_id = None;
            cx.notify();
            return;
        }
        self.api_providers.order_saving = true;
        cx.notify();
        match self.workspace.reorder_api_providers(&merged) {
            Ok(()) => {
                // Optimistic local sort_index + list order.
                let mut by_id: std::collections::HashMap<String, ApiProvider> = self
                    .api_providers
                    .items
                    .drain(..)
                    .map(|p| (p.id.clone(), p))
                    .collect();
                let mut next = Vec::with_capacity(merged.len());
                for (i, id) in merged.iter().enumerate() {
                    if let Some(mut p) = by_id.remove(id) {
                        p.sort_index = i as i64;
                        next.push(p);
                    }
                }
                // Keep any leftovers (should be none) at the end.
                for (_, mut p) in by_id {
                    p.sort_index = next.len() as i64;
                    next.push(p);
                }
                self.api_providers.items = next;
                self.api_providers.order_saving = false;
                self.api_providers.dragging_id = None;
                self.api_providers.drop_hover_id = None;
                self.api_providers.settled_id = highlight_id.map(str::to_string);
                self.api_providers.status_message =
                    Some(t!("api_providers.order_saved").to_string());
                cx.notify();
                cx.spawn(
                    move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
                        let mut cx = cx.clone();
                        async move {
                            cx.background_executor()
                                .timer(std::time::Duration::from_millis(450))
                                .await;
                            let _ = this.update(&mut cx, |this, cx| {
                                this.api_providers.settled_id = None;
                                cx.notify();
                            });
                        }
                    },
                )
                .detach();
            }
            Err(err) => {
                self.api_providers.order_saving = false;
                self.api_providers.dragging_id = None;
                self.api_providers.drop_hover_id = None;
                self.api_providers.status_message = Some(err.to_string());
                self.refresh_api_providers(cx);
            }
        }
    }

    /// Move `id` before/after a drop target within the current filtered view.
    pub(crate) fn move_api_provider_before(
        &mut self,
        source_id: &str,
        target_id: &str,
        cx: &mut Context<Self>,
    ) {
        if source_id == target_id || self.api_providers.order_saving {
            self.api_providers.dragging_id = None;
            cx.notify();
            return;
        }
        let visible: Vec<String> = self.filtered_api_providers_ids(cx);
        let Some(from) = visible.iter().position(|id| id == source_id) else {
            return;
        };
        let Some(to) = visible.iter().position(|id| id == target_id) else {
            return;
        };
        let mut next = visible;
        let item = next.remove(from);
        next.insert(to, item);
        self.apply_api_provider_reorder(&next, Some(source_id), cx);
    }

    /// Nudge a provider up/down within the filtered list (ArrowUp / ArrowDown).
    pub(crate) fn nudge_api_provider(&mut self, id: &str, delta: isize, cx: &mut Context<Self>) {
        if delta == 0 || self.api_providers.order_saving {
            return;
        }
        let visible: Vec<String> = self.filtered_api_providers_ids(cx);
        if visible.len() < 2 {
            return;
        }
        let Some(from) = visible.iter().position(|x| x == id) else {
            return;
        };
        let to = from as isize + delta;
        if to < 0 || to as usize >= visible.len() {
            return;
        }
        let mut next = visible;
        next.swap(from, to as usize);
        self.apply_api_provider_reorder(&next, Some(id), cx);
    }

    fn filtered_api_providers_ids(&self, cx: &Context<Self>) -> Vec<String> {
        self.filtered_api_providers(cx)
            .into_iter()
            .map(|p| p.id)
            .collect()
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
            ApiProviderKind::CodexSubscription => ApiAuthFlow::Browser,
            _ => ApiAuthFlow::Unset,
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
        let can_reorder = rows.len() >= 2 && !self.api_providers.order_saving;
        let global_rank: std::collections::HashMap<String, usize> = self
            .api_providers
            .items
            .iter()
            .enumerate()
            .map(|(i, p)| (p.id.clone(), i + 1))
            .collect();
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
                            .child(div().w(px(96.)).child("#"))
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
                                    .w(px(180.))
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
                                    .w(px(120.))
                                    .child(t!("api_providers.col_actions").to_string()),
                            ),
                    )
                    .child(v_flex().flex_1().min_h_0().overflow_y_scrollbar().children(
                        rows.into_iter().map(|p| {
                            let rank = global_rank.get(&p.id).copied().unwrap_or(0);
                            self.render_api_provider_row(rank, p, can_reorder, zh, cx)
                        }),
                    ))
                    .into_any_element()
            })
            .when(self.api_providers.order_saving, |el| {
                el.child(
                    div()
                        .px(px(24.))
                        .py(px(6.))
                        .text_size(px(12.))
                        .text_color(theme.muted_foreground)
                        .child(t!("api_providers.order_saving").to_string()),
                )
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
        global_index: usize,
        p: ApiProvider,
        can_reorder: bool,
        zh: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme().clone();
        let dark = theme.is_dark();
        let id = p.id.clone();
        let id_edit = p.id.clone();
        let id_toggle = p.id.clone();
        let id_del = p.id.clone();
        let id_drag = p.id.clone();
        let id_drop = p.id.clone();
        let id_up = p.id.clone();
        let id_down = p.id.clone();
        let kind_label = p.kind_label(zh).to_string();
        let model_count = p.models.len();
        let cap_count = p.capabilities.len();
        let drag_label = SharedString::from(p.name.clone());
        let plan_badge = self
            .api_providers
            .usage_by_id
            .get(&p.id)
            .and_then(|u| u.plan_type.as_deref())
            .filter(|s| !s.is_empty())
            .map(session::plan_type_label);
        let account_line = match (
            &p.kind,
            p.subscription.as_ref().and_then(|s| s.account_hint.clone()),
        ) {
            (ApiProviderKind::CodexSubscription, Some(hint)) if !hint.is_empty() => {
                t!("api_providers.openai_account", hint = hint).to_string()
            }
            (ApiProviderKind::ClaudeSubscription, Some(hint)) if !hint.is_empty() => {
                t!("api_providers.claude_account", hint = hint).to_string()
            }
            (ApiProviderKind::GrokSubscription, Some(hint)) if !hint.is_empty() => {
                t!("api_providers.xai_account", hint = hint).to_string()
            }
            _ => p.connection_hint(),
        };
        let usage_snapshot = self.api_providers.usage_by_id.get(&p.id).cloned();
        let usage_loading = self.api_providers.usage_loading.contains(&p.id);
        let is_dragging = self.api_providers.dragging_id.as_deref() == Some(p.id.as_str());
        let is_drop_hover =
            self.api_providers.drop_hover_id.as_deref() == Some(p.id.as_str()) && !is_dragging;
        let is_settled = self.api_providers.settled_id.as_deref() == Some(p.id.as_str());
        let kind_label_ghost = SharedString::from(kind_label.clone());
        let self_id_for_drag_over = p.id.clone();
        let show_drop_slot = is_dragging || is_drop_hover;

        h_flex()
            .id(SharedString::from(format!("api-row-{}", p.id)))
            .w_full()
            .px(px(24.))
            .py(px(14.))
            .gap(px(8.))
            .items_center()
            .rounded(px(8.))
            .relative()
            .when(!show_drop_slot && !is_settled, |row| {
                row.rounded(px(0.))
                    .border_b_1()
                    .border_color(theme.border.opacity(0.6))
            })
            .when(show_drop_slot, |row| {
                // AstrLink drop slot: dashed primary frame + soft fill + left accent.
                row.border_2()
                    .border_dashed()
                    .border_color(astrlink_primary(0.6))
                    .bg(astrlink_primary(0.05))
                    .when(is_dragging, |r| r.opacity(0.55))
            })
            .when(is_settled, |row| {
                // Brief settle pulse after drop (AstrLink spring land).
                row.border_1()
                    .border_color(astrlink_primary(0.6))
                    .bg(astrlink_primary(0.06))
                    .shadow_lg()
            })
            .hover(|s| {
                if show_drop_slot {
                    s
                } else {
                    s.bg(theme.secondary.opacity(0.35))
                }
            })
            .when(can_reorder, |row| {
                let hover_id = self_id_for_drag_over.clone();
                row.on_drag_move(cx.listener(
                    move |this, ev: &gpui::DragMoveEvent<DragApiProviderId>, _, cx| {
                        let source = &ev.drag(cx).0;
                        if source == &hover_id {
                            if this.api_providers.drop_hover_id.is_some() {
                                this.api_providers.drop_hover_id = None;
                                cx.notify();
                            }
                            return;
                        }
                        if this.api_providers.drop_hover_id.as_deref() != Some(hover_id.as_str()) {
                            this.api_providers.drop_hover_id = Some(hover_id.clone());
                            cx.notify();
                        }
                    },
                ))
                .drag_over::<DragApiProviderId>(move |style, dragged, _, _| {
                    if dragged.0 == self_id_for_drag_over {
                        return style;
                    }
                    // Keep style-layer slot in sync (backup for first paint).
                    style
                        .border_2()
                        .border_dashed()
                        .border_color(astrlink_primary(0.6))
                        .bg(astrlink_primary(0.05))
                        .rounded(px(8.))
                })
                .on_drop(cx.listener(
                    move |this, dragged: &DragApiProviderId, _, cx| {
                        this.move_api_provider_before(&dragged.0, &id_drop, cx);
                    },
                ))
            })
            // AstrLink `before:` left accent bar on drop slot / settle.
            .when(show_drop_slot || is_settled, |row| {
                row.child(
                    div()
                        .absolute()
                        .left(px(-1.))
                        .top(px(6.))
                        .bottom(px(6.))
                        .w(px(4.))
                        .rounded_full()
                        .bg(Hsla::from(rgb(ASTRLINK_PRIMARY))),
                )
            })
            .child(
                h_flex()
                    .w(px(96.))
                    .gap(px(2.))
                    .items_center()
                    .child(
                        div()
                            .id(SharedString::from(format!("api-grip-{}", p.id)))
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(px(28.))
                            .rounded(px(6.))
                            .text_color(if is_dragging {
                                Hsla::from(rgb(ASTRLINK_PRIMARY))
                            } else {
                                theme.muted_foreground
                            })
                            .when(is_dragging, |el| {
                                el.bg(astrlink_primary(0.12))
                                    .border_1()
                                    .border_color(astrlink_primary(0.6))
                            })
                            .when(can_reorder, |el| {
                                let view = cx.entity().downgrade();
                                let drag_id = id_drag.clone();
                                let name = drag_label.clone();
                                let kind = kind_label_ghost.clone();
                                el.cursor_grab()
                                    .hover(|s| s.bg(theme.secondary.opacity(0.8)))
                                    .on_drag(DragApiProviderId(id_drag), move |_, _, _, cx| {
                                        let name = name.clone();
                                        let kind_label = kind.clone();
                                        let ghost =
                                            cx.new(|_| ApiProviderDragGhost { name, kind_label });
                                        if let Some(entity) = view.upgrade() {
                                            let id = drag_id.clone();
                                            entity.update(cx, |this, cx| {
                                                this.api_providers.dragging_id = Some(id);
                                                this.api_providers.drop_hover_id = None;
                                                this.api_providers.settled_id = None;
                                                // Clear placeholder if drag cancelled (ghost released).
                                                cx.observe_release(&ghost, |this, _, cx| {
                                                    if this.api_providers.dragging_id.is_some()
                                                        && !this.api_providers.order_saving
                                                    {
                                                        this.api_providers.dragging_id = None;
                                                        this.api_providers.drop_hover_id = None;
                                                        cx.notify();
                                                    }
                                                })
                                                .detach();
                                                cx.notify();
                                            });
                                        }
                                        ghost
                                    })
                            })
                            .when(!can_reorder, |el| el.opacity(0.35))
                            .child(Icon::new(CustomIcon::GripVertical).size(px(16.))),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme.muted_foreground)
                            .min_w(px(16.))
                            .child(format!("{global_index}")),
                    )
                    .when(can_reorder, |el| {
                        el.child(
                            Button::new(SharedString::from(format!("api-up-{}", p.id)))
                                .ghost()
                                .xsmall()
                                .icon(IconName::ChevronUp)
                                .tooltip(t!("api_providers.order_move_up").to_string())
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.nudge_api_provider(&id_up, -1, cx);
                                })),
                        )
                        .child(
                            Button::new(SharedString::from(format!("api-down-{}", p.id)))
                                .ghost()
                                .xsmall()
                                .icon(IconName::ChevronDown)
                                .tooltip(t!("api_providers.order_move_down").to_string())
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.nudge_api_provider(&id_down, 1, cx);
                                })),
                        )
                    }),
            )
            .child(
                h_flex()
                    .flex_1()
                    .min_w(px(220.))
                    .gap(px(12.))
                    .items_center()
                    .child(
                        div()
                            .size(px(36.))
                            .rounded(px(8.))
                            .border_1()
                            .border_color(theme.border)
                            .bg(theme.background)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(brand_img(p.kind.brand_id(), dark, px(22.))),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w(px(0.))
                            .gap(px(2.))
                            .child(
                                h_flex()
                                    .gap(px(8.))
                                    .items_center()
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
                                    .when_some(plan_badge, |el, plan| {
                                        el.child(
                                            div()
                                                .px(px(6.))
                                                .py(px(1.))
                                                .rounded(px(4.))
                                                .bg(theme.secondary)
                                                .text_size(px(11.))
                                                .text_color(theme.muted_foreground)
                                                .child(plan),
                                        )
                                    }),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(theme.muted_foreground)
                                    .child(kind_label),
                            )
                            .child(
                                div()
                                    .max_w(px(280.))
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .text_size(px(12.))
                                    .text_color(theme.muted_foreground)
                                    .child(account_line),
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
            .child(div().w(px(180.)).child(self.render_api_provider_usage_cell(
                &p,
                usage_snapshot.as_ref(),
                usage_loading,
                zh,
                cx,
            )))
            .child(
                div()
                    .w(px(100.))
                    .text_size(px(12.))
                    .text_color(theme.muted_foreground)
                    .child(t!("api_providers.billing_cycle_zero").to_string()),
            )
            .child(
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
                    .w(px(120.))
                    .gap(px(2.))
                    .items_center()
                    // 连通性测试（AstrLink Flask / ServiceTest）暂未接入，先隐藏。
                    // .child(
                    //     Button::new(SharedString::from(format!("api-test-{}", p.id)))
                    //         .ghost()
                    //         .small()
                    //         .icon(CustomIcon::Flask)
                    //         .tooltip(t!("api_providers.test").to_string())
                    //         .on_click(cx.listener(move |this, _, _, cx| {
                    //             this.api_providers.status_message =
                    //                 Some(t!("api_providers.test_todo").to_string());
                    //             cx.notify();
                    //         })),
                    // )
                    .child(
                        Button::new(SharedString::from(format!("api-edit-{}", p.id)))
                            .ghost()
                            .small()
                            .icon(CustomIcon::SquarePen)
                            .tooltip(t!("api_providers.edit").to_string())
                            .on_click(cx.listener({
                                let id = id.clone();
                                move |this, _, window, cx| {
                                    this.open_api_providers_edit(&id, window, cx);
                                }
                            })),
                    )
                    .child({
                        let entity = cx.entity();
                        let id_more = id_del.clone();
                        Button::new(SharedString::from(format!("api-more-{}", p.id)))
                            .ghost()
                            .small()
                            .icon(IconName::Menu)
                            .tooltip(t!("api_providers.more").to_string())
                            .dropdown_menu_with_anchor(Corner::TopRight, move |menu, _, _| {
                                let e = entity.clone();
                                let id = id_more.clone();
                                menu.min_w(px(140.)).item(
                                    PopupMenuItem::new(t!("api_providers.delete").to_string())
                                        .on_click(move |_, _, cx| {
                                            e.update(cx, |this, cx| {
                                                this.delete_api_provider(&id, cx);
                                            });
                                        }),
                                )
                            })
                    }),
            )
    }

    fn render_api_provider_usage_cell(
        &self,
        p: &ApiProvider,
        usage: Option<&session::CodexSubscriptionUsage>,
        loading: bool,
        zh: bool,
        cx: &Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme().clone();
        if !(p.kind.is_subscription()
            || matches!(
                p.kind,
                ApiProviderKind::OpencodeGo
                    | ApiProviderKind::KimiCoding
                    | ApiProviderKind::GlmCoding
                    | ApiProviderKind::MinimaxCoding
                    | ApiProviderKind::Newapi
            ))
        {
            return div().into_any_element();
        }
        let connected = p.subscription.as_ref().is_some_and(|s| {
            matches!(
                s.status,
                SubscriptionStatus::Connected | SubscriptionStatus::NeedsReauth
            )
        });
        let usage_error = self.api_providers.usage_error_by_id.get(&p.id);
        if !connected {
            return div()
                .text_size(px(12.))
                .text_color(theme.muted_foreground)
                .child(t!("api_providers.usage_na").to_string())
                .into_any_element();
        }
        if let Some(err) = usage_error {
            return div()
                .text_size(px(12.))
                .text_color(rgb(0xB45309))
                .child(err.clone())
                .into_any_element();
        }
        if loading && usage.is_none() {
            return div()
                .text_size(px(12.))
                .text_color(theme.muted_foreground)
                .child(t!("api_providers.loading").to_string())
                .into_any_element();
        }
        let Some(usage) = usage else {
            return div()
                .text_size(px(12.))
                .text_color(theme.muted_foreground)
                .child(t!("api_providers.usage_na").to_string())
                .into_any_element();
        };
        let window = usage.primary.as_ref().or(usage.secondary.as_ref());
        let Some(window) = window else {
            return div()
                .text_size(px(12.))
                .text_color(theme.muted_foreground)
                .child(t!("api_providers.usage_na").to_string())
                .into_any_element();
        };
        let used = window.used_percent.clamp(0.0, 100.0);
        let remaining = (100.0 - used).clamp(0.0, 100.0);
        let bar = (remaining / 100.0) as f32;
        let label = session::window_label(window.limit_window_seconds, false, zh);
        let reset = session::format_reset_countdown(window, zh)
            .unwrap_or_else(|| t!("api_providers.usage_na").to_string());
        let tone_ok = used < 80.0 && !usage.limit_reached;
        let tone_warn = used >= 80.0 && used < 100.0 && !usage.limit_reached;
        let bar_color = if usage.limit_reached || used >= 100.0 {
            rgb(0xDC2626)
        } else if tone_warn {
            rgb(0xD97706)
        } else {
            rgb(0x16A34A)
        };
        let pct_color = if tone_ok {
            rgb(0x15803D)
        } else if tone_warn {
            rgb(0xB45309)
        } else {
            rgb(0xB91C1C)
        };

        v_flex()
            .w_full()
            .gap(px(4.))
            .child(
                h_flex()
                    .w_full()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.foreground)
                            .child(label),
                    )
                    .child(
                        div().text_size(px(12.)).text_color(pct_color).child(
                            t!(
                                "api_providers.remaining_percent",
                                percent = format!("{:.0}", remaining)
                            )
                            .to_string(),
                        ),
                    ),
            )
            .child(
                div()
                    .w_full()
                    .h(px(6.))
                    .rounded(px(3.))
                    .bg(theme.secondary)
                    .child(
                        div()
                            .h_full()
                            .rounded(px(3.))
                            .bg(bar_color)
                            .w(relative(bar)),
                    ),
            )
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(theme.muted_foreground)
                    .child(reset),
            )
            .into_any_element()
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
        let needs_reauth = draft.as_ref().is_some_and(|d| {
            d.subscription.as_ref().is_some_and(|s| {
                matches!(
                    s.status,
                    SubscriptionStatus::NeedsReauth
                        | SubscriptionStatus::Disconnected
                        | SubscriptionStatus::Error
                )
            })
        });
        let save_label = if creating && is_sub {
            t!("api_providers.add_and_login").to_string()
        } else if !creating && is_sub && needs_reauth {
            t!("api_providers.add_and_login").to_string()
        } else if creating {
            t!("api_providers.save_service").to_string()
        } else {
            t!("api_providers.save_changes").to_string()
        };
        let save_disabled = (creating || needs_reauth)
            && matches!(kind, Some(ApiProviderKind::CodexSubscription))
            && matches!(self.api_providers.auth_flow, ApiAuthFlow::Unset);

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
                            None,
                            cx,
                        ))
                        .child(self.render_tab_divider(cx))
                        .child(self.render_edit_tab(
                            "tab-models",
                            ApiProvidersEditTab::Models,
                            IconName::Building2,
                            t!("api_providers.tab_models").to_string(),
                            draft.as_ref().map(|d| {
                                format!("{} {}", d.models.len(), t!("api_providers.models_unit"))
                            }),
                            cx,
                        ))
                        .child(self.render_tab_divider(cx))
                        .child(self.render_edit_tab(
                            "tab-proto",
                            ApiProvidersEditTab::Protocols,
                            IconName::Settings2,
                            t!("api_providers.tab_protocols").to_string(),
                            draft.as_ref().map(|d| {
                                format!(
                                    "{} {}",
                                    d.capabilities.len(),
                                    t!("api_providers.caps_enabled_unit")
                                )
                            }),
                            cx,
                        ))
                        .child(self.render_tab_divider(cx))
                        .child(self.render_edit_tab(
                            "tab-fail",
                            ApiProvidersEditTab::Failure,
                            IconName::TriangleAlert,
                            t!("api_providers.tab_failure").to_string(),
                            None,
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
                                    .disabled(save_disabled)
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
        badge: Option<String>,
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
            .when_some(badge, |el, text| {
                el.child(
                    div()
                        .px(px(6.))
                        .py(px(1.))
                        .rounded(px(999.))
                        .bg(theme.secondary)
                        .text_size(px(10.))
                        .text_color(theme.muted_foreground)
                        .child(text),
                )
            })
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
                d.kind.supports_http_model_probe()
                    || d.kind.is_subscription()
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
                                .outline()
                                .label(if fetching {
                                    t!("api_providers.models_fetching").to_string()
                                } else {
                                    t!("api_providers.models_fetch").to_string()
                                })
                                .disabled(fetching)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.fetch_draft_api_provider_models(window, cx);
                                })),
                        )
                    })
                    .child(
                        Button::new("api-models-add-toggle")
                            .outline()
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

    fn toggle_draft_protocol(&mut self, protocol_id: &str, cx: &mut Context<Self>) {
        let Some(draft) = self.api_providers.draft.as_mut() else {
            return;
        };
        let natives = draft.kind.subscription_native_protocol_ids();
        if natives.contains(&protocol_id) {
            return;
        }
        if let Some(idx) = draft
            .capabilities
            .iter()
            .position(|c| c.protocol == protocol_id)
        {
            draft.capabilities.remove(idx);
        } else {
            let streaming = domain::api_protocol_descriptor(protocol_id)
                .map(|d| d.streaming)
                .unwrap_or(true);
            let convert_to = draft
                .kind
                .subscription_conversion_targets()
                .first()
                .map(|s| (*s).to_string());
            if let Some(target) = convert_to {
                draft
                    .capabilities
                    .push(ApiCapability::converted(protocol_id, streaming, target));
            } else {
                draft
                    .capabilities
                    .push(ApiCapability::native(protocol_id, streaming));
            }
        }
        draft
            .kind
            .ensure_subscription_natives(&mut draft.capabilities);
        cx.notify();
    }

    fn render_protocols_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let draft = self.api_providers.draft.clone();
        let kind = draft.as_ref().map(|d| d.kind);
        let caps = draft
            .as_ref()
            .map(|d| d.capabilities.clone())
            .unwrap_or_default();
        let is_sub = kind.map(|k| k.is_subscription()).unwrap_or(false);
        let natives: std::collections::HashSet<&str> = kind
            .map(|k| {
                k.subscription_native_protocol_ids()
                    .iter()
                    .copied()
                    .collect()
            })
            .unwrap_or_default();
        let targets = kind
            .map(|k| k.subscription_conversion_targets())
            .unwrap_or(&[]);
        let target_labels = targets
            .iter()
            .filter_map(|id| domain::api_protocol_descriptor(id).map(|d| d.label))
            .collect::<Vec<_>>()
            .join(" / ");
        let rows = kind.map(|k| k.protocol_editor_rows()).unwrap_or_default();
        let row_count = rows.len();

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
                    .pb(px(12.))
                    .gap(px(4.))
                    .border_b_1()
                    .border_color(theme.border)
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(t!("api_providers.capabilities_title").to_string()),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(theme.muted_foreground)
                                    .child(t!("api_providers.protocol_mode_help").to_string()),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme.muted_foreground)
                            .child(if is_sub && !target_labels.is_empty() {
                                t!(
                                    "api_providers.capability_hint_subscription",
                                    targets = target_labels
                                )
                                .to_string()
                            } else if is_sub {
                                t!("api_providers.capability_hint_subscription_plain").to_string()
                            } else {
                                t!("api_providers.capability_hint_http").to_string()
                            }),
                    ),
            )
            .children(rows.into_iter().enumerate().map(|(i, desc)| {
                let protocol_id = desc.id.to_string();
                let enabled = caps.iter().any(|c| c.protocol == desc.id);
                let native = natives.contains(desc.id);
                let locked = native;
                let convert_note = caps
                    .iter()
                    .find(|c| c.protocol == desc.id)
                    .and_then(|c| c.convert_to.as_deref())
                    .and_then(|id| domain::api_protocol_descriptor(id).map(|d| d.label));
                h_flex()
                    .id(SharedString::from(format!("proto-row-{protocol_id}")))
                    .w_full()
                    .px(px(16.))
                    .py(px(12.))
                    .gap(px(12.))
                    .items_center()
                    .when(i + 1 < row_count, |el| {
                        el.border_b_1().border_color(theme.border.opacity(0.7))
                    })
                    .child(
                        Checkbox::new(SharedString::from(format!("proto-cb-{protocol_id}")))
                            .checked(enabled)
                            .disabled(locked)
                            .on_click(cx.listener({
                                let protocol_id = protocol_id.clone();
                                move |this, checked: &bool, _, cx| {
                                    let Some(draft) = this.api_providers.draft.as_mut() else {
                                        return;
                                    };
                                    let natives = draft.kind.subscription_native_protocol_ids();
                                    if natives.contains(&protocol_id.as_str()) {
                                        return;
                                    }
                                    let currently = draft
                                        .capabilities
                                        .iter()
                                        .any(|c| c.protocol == protocol_id);
                                    if *checked == currently {
                                        return;
                                    }
                                    this.toggle_draft_protocol(&protocol_id, cx);
                                }
                            })),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w(px(0.))
                            .gap(px(2.))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(theme.foreground)
                                    .child(desc.label.to_string()),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .font_family("Menlo")
                                    .text_color(theme.muted_foreground)
                                    .child(desc.entry_path.to_string()),
                            )
                            .when_some(convert_note, |el, label| {
                                el.child(
                                    div().text_size(px(11.)).text_color(rgb(0x2563EB)).child(
                                        t!("api_providers.convert_to_label", target = label)
                                            .to_string(),
                                    ),
                                )
                            }),
                    )
                    .when(native, |el| {
                        el.child(
                            div()
                                .text_size(px(12.))
                                .text_color(theme.muted_foreground)
                                .child(t!("api_providers.protocol_native").to_string()),
                        )
                    })
            }))
    }
}
