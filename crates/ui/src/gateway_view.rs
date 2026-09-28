//! Gateway page with AstrLink-aligned access token management.

use domain::{
    AccessTokenSummary, AccessTokenUsage, AppKind, ClaudeForm, ClaudeKind, ClaudeModelMapping,
    CodexForm, CodexKind, OpenCodeForm, OpenCodeKind, ProviderForm, RequestProtocol,
    DEFAULT_CLAUDE_MODEL, DEFAULT_CODEX_MODEL, DEFAULT_OPENCODE_MODEL,
};
use gpui::{
    div, img, prelude::FluentBuilder, px, rgb, AppContext, Context, Entity, FontWeight, Hsla,
    InteractiveElement, IntoElement, ParentElement, SharedString, StatefulInteractiveElement,
    Styled, Window,
};
use gpui_component::{
    button::{Button, ButtonCustomVariant, ButtonVariants as _},
    h_flex,
    input::{Input, InputState},
    scroll::ScrollableElement as _,
    v_flex, ActiveTheme, Disableable as _, Icon, IconName, Sizable as _,
};
use rust_i18n::t;
use session::access_token::format_created_at;

use crate::api_providers_view::astrlink_primary_btn;
use crate::app_view::{notify_success, RouterApp};
use crate::assets::{brand_img, CustomIcon};

const ASTRLINK_PRIMARY: u32 = 0x1D4D87;
const CC_SWITCH_HOVER_BG: u32 = 0xE8F1FF;
const CC_SWITCH_HOVER_FG: u32 = 0x1D4D87;

#[derive(Debug, Clone, PartialEq, Eq)]
enum GatewayPane {
    List,
    Create,
    Import(String),
}

impl GatewayPane {
    fn import_token_id(&self) -> Option<&str> {
        match self {
            Self::Import(id) => Some(id.as_str()),
            _ => None,
        }
    }
}

pub struct GatewayState {
    pub items: Vec<AccessTokenSummary>,
    pub loading: bool,
    pub error: Option<String>,
    pub pane: GatewayPane,
    pub search: Entity<InputState>,
    pub create_name: Entity<InputState>,
    pub creating: bool,
    pub deleting_id: Option<String>,
    pub pending_delete: Option<AccessTokenSummary>,
    pub copying_id: Option<String>,
    pub copied_id: Option<String>,
    pub usage_by_id: std::collections::HashMap<String, AccessTokenUsage>,
    pub usage_loading: bool,
    pub import_app: AppKind,
    pub import_name: Entity<InputState>,
    pub import_model: Entity<InputState>,
    pub import_haiku: Entity<InputState>,
    pub import_sonnet: Entity<InputState>,
    pub import_opus: Entity<InputState>,
    pub importing: bool,
}

impl GatewayState {
    pub fn new(window: &mut Window, cx: &mut Context<RouterApp>) -> Self {
        let model_ph = t!("gateway.import_model_placeholder").to_string();
        Self {
            items: Vec::new(),
            loading: false,
            error: None,
            pane: GatewayPane::List,
            search: cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder(t!("gateway.search_placeholder").to_string())
            }),
            create_name: cx.new(|cx| {
                InputState::new(window, cx).placeholder(t!("gateway.name_placeholder").to_string())
            }),
            creating: false,
            deleting_id: None,
            pending_delete: None,
            copying_id: None,
            copied_id: None,
            usage_by_id: std::collections::HashMap::new(),
            usage_loading: false,
            import_app: AppKind::Claude,
            import_name: cx.new(|cx| InputState::new(window, cx)),
            import_model: cx.new(|cx| InputState::new(window, cx).placeholder(model_ph.clone())),
            import_haiku: cx.new(|cx| InputState::new(window, cx).placeholder(model_ph.clone())),
            import_sonnet: cx.new(|cx| InputState::new(window, cx).placeholder(model_ph.clone())),
            import_opus: cx.new(|cx| InputState::new(window, cx).placeholder(model_ph)),
            importing: false,
        }
    }
}

impl RouterApp {
    pub(crate) fn refresh_gateway_tokens(&mut self, cx: &mut Context<Self>) {
        self.gateway.loading = true;
        self.gateway.error = None;
        cx.notify();
        match self.workspace.list_access_tokens() {
            Ok(items) => {
                self.gateway.items = items;
                self.refresh_gateway_token_usage(cx);
            }
            Err(err) => self.gateway.error = Some(err.to_string()),
        }
        self.gateway.loading = false;
        cx.notify();
    }

    fn refresh_gateway_token_usage(&mut self, cx: &mut Context<Self>) {
        let ids: Vec<String> = self.gateway.items.iter().map(|t| t.id.clone()).collect();
        if ids.is_empty() {
            self.gateway.usage_by_id.clear();
            return;
        }
        self.gateway.usage_loading = true;
        match self.workspace.list_access_token_usage(&ids) {
            Ok(rows) => {
                self.gateway.usage_by_id =
                    rows.into_iter().map(|u| (u.token_id.clone(), u)).collect();
            }
            Err(err) => self.gateway.error = Some(err.to_string()),
        }
        self.gateway.usage_loading = false;
        cx.notify();
    }

    pub(crate) fn render_gateway_page(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_gateway_list(cx)
    }

    fn gateway_api_url(&self) -> String {
        self.workspace
            .gateway_api_url()
            .unwrap_or_else(|| "http://127.0.0.1:8787".to_string())
    }

    fn gateway_ready(&self) -> bool {
        self.workspace.is_gateway_ready()
    }

    fn filtered_gateway_tokens(&self, cx: &Context<Self>) -> Vec<AccessTokenSummary> {
        let q = self.gateway.search.read(cx).value().trim().to_lowercase();
        if q.is_empty() {
            return self.gateway.items.clone();
        }
        self.gateway
            .items
            .iter()
            .filter(|token| {
                format!("{} {}", token.name, token.hint)
                    .to_lowercase()
                    .contains(&q)
            })
            .cloned()
            .collect()
    }

    fn render_gateway_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let api_url = self.gateway_api_url();
        let rows = self.filtered_gateway_tokens(cx);
        let total = self.gateway.items.len();
        let ready = self.gateway_ready();
        let busy =
            self.gateway.loading || self.gateway.creating || self.gateway.deleting_id.is_some();
        let search_q = self.gateway.search.read(cx).value().trim().to_string();

        v_flex()
            .size_full()
            .relative()
            .bg(theme.background)
            .child(
                h_flex()
                    .w_full()
                    .px(px(24.))
                    .pt(px(20.))
                    .pb(px(12.))
                    .items_center()
                    .justify_between()
                    .child(
                        v_flex()
                            .gap(px(4.))
                            .child(
                                div()
                                    .text_size(px(22.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(theme.foreground)
                                    .child(t!("gateway.title").to_string()),
                            )
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(theme.muted_foreground)
                                    .child(t!("gateway.description").to_string()),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap(px(8.))
                            .items_center()
                            .child(
                                Button::new("gateway-create")
                                    .small()
                                    .custom(astrlink_primary_btn(cx))
                                    .icon(IconName::Plus)
                                    .label(t!("gateway.create_token").to_string())
                                    .disabled(!ready || busy)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.gateway.pane = GatewayPane::Create;
                                        this.gateway.error = None;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("gateway-refresh")
                                    .small()
                                    .outline()
                                    .icon(CustomIcon::RefreshCw)
                                    .label(if self.gateway.loading {
                                        t!("common.refreshing").to_string()
                                    } else {
                                        t!("common.refresh").to_string()
                                    })
                                    .disabled(!ready || busy)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.refresh_gateway_tokens(cx);
                                    })),
                            ),
                    ),
            )
            .child(
                h_flex()
                    .mx(px(24.))
                    .mb(px(12.))
                    .px(px(14.))
                    .py(px(10.))
                    .rounded(px(8.))
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.muted.opacity(0.35))
                    .items_center()
                    .justify_between()
                    .child(
                        h_flex()
                            .gap(px(8.))
                            .items_center()
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(theme.muted_foreground)
                                    .child(t!("gateway.api_address").to_string()),
                            )
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(theme.foreground)
                                    .child(if ready {
                                        api_url.clone()
                                    } else {
                                        t!("gateway.waiting_ready").to_string()
                                    }),
                            ),
                    )
                    .child(
                        Button::new("gateway-copy-api")
                            .ghost()
                            .xsmall()
                            .icon(IconName::Copy)
                            .tooltip(t!("gateway.copy_api_address").to_string())
                            .disabled(!ready)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                let _ = this;
                                cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                    api_url.clone(),
                                ));
                                notify_success(
                                    t!("gateway.api_address_copied").to_string(),
                                    window,
                                    cx,
                                );
                            })),
                    ),
            )
            .when(!ready, |el| {
                el.child(
                    div()
                        .mx(px(24.))
                        .mb(px(8.))
                        .px(px(12.))
                        .py(px(8.))
                        .rounded(px(6.))
                        .bg(Hsla::from(rgb(0xFEF3C7)))
                        .text_size(px(13.))
                        .text_color(Hsla::from(rgb(0x92400E)))
                        .child(if total > 0 {
                            t!("gateway.stale").to_string()
                        } else {
                            t!("gateway.blocked").to_string()
                        }),
                )
            })
            .when_some(self.gateway.error.clone(), |el, msg| {
                el.child(
                    div()
                        .mx(px(24.))
                        .mb(px(8.))
                        .px(px(12.))
                        .py(px(8.))
                        .rounded(px(6.))
                        .bg(Hsla::from(rgb(0xFEE2E2)))
                        .text_size(px(13.))
                        .text_color(Hsla::from(rgb(0xB91C1C)))
                        .child(msg),
                )
            })
            .child(
                h_flex()
                    .w_full()
                    .px(px(24.))
                    .pb(px(10.))
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(14.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.foreground)
                            .child(format!(
                                "{} ({})",
                                t!("gateway.list_label"),
                                if rows.len() != total && !search_q.is_empty() {
                                    format!("{} / {}", rows.len(), total)
                                } else {
                                    total.to_string()
                                }
                            )),
                    )
                    .child(
                        div().w(px(288.)).child(
                            Input::new(&self.gateway.search)
                                .small()
                                .cleanable(true)
                                .disabled(!ready)
                                .prefix(
                                    Icon::new(IconName::Search)
                                        .size(px(14.))
                                        .text_color(theme.muted_foreground),
                                ),
                        ),
                    ),
            )
            .child(
                v_flex()
                    .flex_1()
                    .min_h(px(0.))
                    .px(px(24.))
                    .pb(px(24.))
                    .overflow_y_scrollbar()
                    .when(
                        self.gateway.loading && self.gateway.items.is_empty(),
                        |el| {
                            el.children((0..3).map(|i| {
                                div()
                                    .id(SharedString::from(format!("gateway-skeleton-{i}")))
                                    .mb(px(8.))
                                    .h(px(76.))
                                    .rounded(px(8.))
                                    .bg(theme.muted.opacity(0.5))
                            }))
                        },
                    )
                    .when(
                        !self.gateway.loading && self.gateway.items.is_empty(),
                        |el| {
                            el.child(if ready {
                                self.render_gateway_empty(cx, busy).into_any_element()
                            } else {
                                self.render_gateway_waiting(cx).into_any_element()
                            })
                        },
                    )
                    .when(!rows.is_empty(), |el| {
                        el.children(
                            rows.iter().map(|token| {
                                self.render_gateway_token_row(token, !ready || busy, cx)
                            }),
                        )
                    })
                    .when(
                        !self.gateway.loading && !self.gateway.items.is_empty() && rows.is_empty(),
                        |el| {
                            el.child(
                                div()
                                    .py(px(32.))
                                    .text_center()
                                    .text_color(theme.muted_foreground)
                                    .child(t!("gateway.no_search_results").to_string()),
                            )
                        },
                    ),
            )
            .when(matches!(self.gateway.pane, GatewayPane::Create), |el| {
                el.child(self.render_gateway_create_overlay(cx))
            })
            .when(self.gateway.pending_delete.is_some(), |el| {
                el.child(self.render_gateway_delete_overlay(cx))
            })
            .when(matches!(self.gateway.pane, GatewayPane::Import(_)), |el| {
                el.child(self.render_gateway_import_overlay(cx))
            })
    }

    fn render_gateway_waiting(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        v_flex()
            .items_center()
            .justify_center()
            .py(px(48.))
            .gap(px(8.))
            .child(
                div()
                    .text_size(px(16.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.foreground)
                    .child(t!("gateway.waiting").to_string()),
            )
            .child(
                div()
                    .text_size(px(13.))
                    .text_color(theme.muted_foreground)
                    .child(t!("gateway.waiting_hint").to_string()),
            )
    }

    fn render_gateway_empty(&self, cx: &mut Context<Self>, busy: bool) -> impl IntoElement {
        let theme = cx.theme();
        v_flex()
            .items_center()
            .justify_center()
            .py(px(48.))
            .gap(px(12.))
            .child(
                div()
                    .text_size(px(16.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.foreground)
                    .child(t!("gateway.empty").to_string()),
            )
            .child(
                div()
                    .text_size(px(13.))
                    .text_color(theme.muted_foreground)
                    .child(t!("gateway.empty_hint").to_string()),
            )
            .child(
                Button::new("gateway-empty-create")
                    .small()
                    .custom(astrlink_primary_btn(cx))
                    .icon(IconName::Plus)
                    .label(t!("gateway.create_token").to_string())
                    .disabled(busy)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.gateway.pane = GatewayPane::Create;
                        cx.notify();
                    })),
            )
    }

    fn render_gateway_token_row(
        &self,
        token: &AccessTokenSummary,
        busy: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme().clone();
        let usage = self.gateway.usage_by_id.get(&token.id);
        let today = usage.map(|u| u.today_tokens).unwrap_or(0);
        let lifetime = usage.map(|u| u.total_tokens).unwrap_or(0);
        let is_copying = self.gateway.copying_id.as_deref() == Some(token.id.as_str());
        let is_copied = self.gateway.copied_id.as_deref() == Some(token.id.as_str());
        let is_deleting = self.gateway.deleting_id.as_deref() == Some(token.id.as_str());
        let token_id = token.id.clone();
        let token_for_delete = token.clone();
        let token_for_import = token.clone();
        let created = format_created_at(&token.created_at);
        let row_id = SharedString::from(format!("gateway-token-{}", token.id));

        h_flex()
            .id(row_id)
            .mb(px(8.))
            .px(px(14.))
            .py(px(12.))
            .rounded(px(8.))
            .border_1()
            .border_color(theme.border)
            .bg(theme.background)
            .items_center()
            .gap(px(12.))
            .child(
                div()
                    .size(px(36.))
                    .rounded(px(8.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(theme.muted.opacity(0.6))
                    .child(
                        Icon::new(CustomIcon::KeyRound)
                            .size(px(16.))
                            .text_color(theme.muted_foreground),
                    ),
            )
            .child(
                v_flex()
                    .flex_1()
                    .min_w(px(0.))
                    .gap(px(2.))
                    .child(
                        div()
                            .text_size(px(14.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.foreground)
                            .overflow_x_hidden()
                            .child(token.name.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme.muted_foreground)
                            .overflow_x_hidden()
                            .child(token.hint.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(theme.muted_foreground)
                            .child(format!("{} · {created}", t!("gateway.created_at"))),
                    ),
            )
            .child(
                h_flex()
                    .flex_shrink_0()
                    .gap(px(20.))
                    .w(px(160.))
                    .child(self.render_usage_stat(
                        t!("gateway.today_tokens").to_string(),
                        today,
                        self.gateway.usage_loading,
                        &theme,
                    ))
                    .child(self.render_usage_stat(
                        t!("gateway.lifetime_tokens").to_string(),
                        lifetime,
                        self.gateway.usage_loading,
                        &theme,
                    )),
            )
            .child(
                h_flex()
                    .flex_shrink_0()
                    .gap(px(4.))
                    .items_center()
                    .child(
                        // AstrLink ghost: transparent by default, soft blue on hover.
                        h_flex()
                            .id(SharedString::from(format!("gateway-import-{}", token.id)))
                            .h(px(28.))
                            .px(px(10.))
                            .gap(px(6.))
                            .items_center()
                            .rounded(px(6.))
                            .text_color(theme.foreground)
                            .text_size(px(13.))
                            .cursor_pointer()
                            .opacity(if busy || is_deleting { 0.5 } else { 1. })
                            .when(!(busy || is_deleting), |el| {
                                el.hover(|s| {
                                    s.bg(Hsla::from(rgb(CC_SWITCH_HOVER_BG)))
                                        .text_color(Hsla::from(rgb(CC_SWITCH_HOVER_FG)))
                                })
                                .on_click(cx.listener(
                                    move |this, _, window, cx| {
                                        this.open_gateway_import(&token_for_import, window, cx);
                                    },
                                ))
                            })
                            .child(img("brands/cc-switch.png").size(px(16.)).flex_shrink_0())
                            .child("CC Switch"),
                    )
                    .child(
                        Button::new(SharedString::from(format!("gateway-delete-{}", token.id)))
                            .small()
                            .ghost()
                            .icon(IconName::Delete)
                            .label(if is_deleting {
                                t!("gateway.deleting").to_string()
                            } else {
                                t!("common.delete").to_string()
                            })
                            .text_color(Hsla::from(rgb(0xB91C1C)))
                            .disabled(busy || is_deleting)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.gateway.pending_delete = Some(token_for_delete.clone());
                                this.gateway.error = None;
                                cx.notify();
                            })),
                    )
                    .child(
                        // Fixed width so "已复制" does not shift usage stats.
                        Button::new(SharedString::from(format!("gateway-copy-{}", token.id)))
                            .small()
                            .outline()
                            .w(px(96.))
                            .icon(if is_copied {
                                IconName::Check
                            } else if is_copying {
                                IconName::LoaderCircle
                            } else {
                                IconName::Copy
                            })
                            .label(if is_copying {
                                t!("common.copying").to_string()
                            } else if is_copied {
                                t!("common.copied").to_string()
                            } else {
                                t!("common.copy").to_string()
                            })
                            .disabled(busy || is_deleting || is_copying)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.copy_gateway_token(token_id.clone(), window, cx);
                            })),
                    ),
            )
    }

    fn render_usage_stat(
        &self,
        label: String,
        value: i64,
        loading: bool,
        theme: &gpui_component::Theme,
    ) -> impl IntoElement {
        v_flex()
            .flex_1()
            .min_w(px(64.))
            .gap(px(2.))
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(theme.muted_foreground)
                    .child(label),
            )
            .child(
                div()
                    .text_size(px(14.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.foreground)
                    .font_family("ui-monospace")
                    .child(if loading {
                        "…".to_string()
                    } else {
                        value.to_string()
                    }),
            )
    }

    fn open_gateway_import(
        &mut self,
        token: &AccessTokenSummary,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.gateway.import_app = AppKind::Claude;
        self.gateway.import_name.update(cx, |s, cx| {
            s.set_value(format!("Router Switch · {}", token.name), window, cx);
        });
        for field in [
            &self.gateway.import_model,
            &self.gateway.import_haiku,
            &self.gateway.import_sonnet,
            &self.gateway.import_opus,
        ] {
            field.update(cx, |s, cx| s.set_value(String::new(), window, cx));
        }
        self.gateway.pane = GatewayPane::Import(token.id.clone());
        self.gateway.error = None;
        cx.notify();
    }

    fn copy_gateway_token(
        &mut self,
        token_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.gateway.copying_id = Some(token_id.clone());
        self.gateway.copied_id = None;
        self.gateway.error = None;
        cx.notify();
        match self.workspace.reveal_access_token(&token_id) {
            Ok(value) => {
                cx.write_to_clipboard(gpui::ClipboardItem::new_string(value));
                self.gateway.copying_id = None;
                self.gateway.copied_id = Some(token_id);
                notify_success(t!("common.copied").to_string(), window, cx);
            }
            Err(err) => {
                self.gateway.copying_id = None;
                self.gateway.error = Some(err.to_string());
            }
        }
        cx.notify();
    }

    fn render_gateway_create_overlay(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let busy = self.gateway.creating;
        div()
            .absolute()
            .inset_0()
            .bg(Hsla::black().opacity(0.45))
            .flex()
            .items_center()
            .justify_center()
            .child(
                v_flex()
                    .id("gateway-create-modal")
                    .w(px(420.))
                    .p(px(20.))
                    .rounded(px(10.))
                    .bg(theme.background)
                    .border_1()
                    .border_color(theme.border)
                    .gap(px(12.))
                    .child(
                        div()
                            .text_size(px(16.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(t!("gateway.create_title").to_string()),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(theme.muted_foreground)
                            .child(t!("gateway.create_hint").to_string()),
                    )
                    .child(
                        v_flex()
                            .gap(px(6.))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(theme.muted_foreground)
                                    .child(t!("gateway.name").to_string()),
                            )
                            .child(Input::new(&self.gateway.create_name).small()),
                    )
                    .child(
                        h_flex()
                            .justify_end()
                            .gap(px(8.))
                            .child(
                                Button::new("gateway-create-cancel")
                                    .small()
                                    .outline()
                                    .label(t!("common.cancel").to_string())
                                    .disabled(busy)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.gateway.pane = GatewayPane::List;
                                        this.gateway.create_name.update(cx, |s, cx| {
                                            s.set_value(String::new(), window, cx);
                                        });
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("gateway-create-confirm")
                                    .small()
                                    .custom(astrlink_primary_btn(cx))
                                    .label(if busy {
                                        t!("gateway.creating").to_string()
                                    } else {
                                        t!("gateway.create").to_string()
                                    })
                                    .disabled(busy)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.submit_gateway_create(window, cx);
                                    })),
                            ),
                    ),
            )
    }

    fn render_gateway_delete_overlay(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let pending = self.gateway.pending_delete.clone();
        let is_last = self.gateway.items.len() <= 1;
        let deleting = self.gateway.deleting_id.is_some();
        let body = pending.as_ref().map(|t| {
            if is_last {
                t!("gateway.delete_last", name = t.name).to_string()
            } else {
                t!("gateway.delete_body", name = t.name).to_string()
            }
        });

        div()
            .absolute()
            .inset_0()
            .bg(Hsla::black().opacity(0.45))
            .flex()
            .items_center()
            .justify_center()
            .child(
                v_flex()
                    .w(px(420.))
                    .p(px(20.))
                    .rounded(px(10.))
                    .bg(theme.background)
                    .border_1()
                    .border_color(theme.border)
                    .gap(px(12.))
                    .child(
                        div()
                            .text_size(px(16.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(t!("gateway.delete_title").to_string()),
                    )
                    .when_some(body, |el, text| {
                        el.child(
                            div()
                                .text_size(px(13.))
                                .text_color(theme.muted_foreground)
                                .child(text),
                        )
                    })
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(theme.muted_foreground)
                            .child(t!("gateway.irreversible").to_string()),
                    )
                    .child(
                        h_flex()
                            .justify_end()
                            .gap(px(8.))
                            .child(
                                Button::new("gateway-delete-cancel")
                                    .small()
                                    .outline()
                                    .label(t!("common.cancel").to_string())
                                    .disabled(deleting)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.gateway.pending_delete = None;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("gateway-delete-confirm")
                                    .small()
                                    .custom(
                                        ButtonCustomVariant::new(cx)
                                            .color(rgb(0xDC2626).into())
                                            .foreground(Hsla::white())
                                            .border(rgb(0xDC2626).into()),
                                    )
                                    .label(if deleting {
                                        t!("gateway.deleting").to_string()
                                    } else {
                                        t!("gateway.confirm_delete").to_string()
                                    })
                                    .disabled(deleting)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.confirm_gateway_delete(window, cx);
                                    })),
                            ),
                    ),
            )
    }

    fn render_gateway_import_overlay(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let dark = theme.is_dark();
        let is_claude = self.gateway.import_app == AppKind::Claude;
        let clients = [AppKind::Claude, AppKind::Codex, AppKind::OpenCode];
        let token = self
            .gateway
            .pane
            .import_token_id()
            .and_then(|id| self.gateway.items.iter().find(|t| t.id == id).cloned());
        let api_url = {
            let base = self.gateway_api_url();
            match self.gateway.import_app {
                AppKind::Codex | AppKind::OpenCode => {
                    if base.ends_with("/v1") {
                        base
                    } else {
                        format!("{}/v1", base.trim_end_matches('/'))
                    }
                }
                _ => base,
            }
        };
        let token_label = token
            .as_ref()
            .map(|t| format!("{} {}", t.name, t.hint))
            .unwrap_or_default();

        div()
            .absolute()
            .inset_0()
            .bg(Hsla::black().opacity(0.45))
            .flex()
            .items_center()
            .justify_center()
            .child(
                v_flex()
                    .id("gateway-import-modal")
                    .w(px(672.))
                    .max_h(px(720.))
                    .p(px(24.))
                    .rounded(px(10.))
                    .bg(theme.background)
                    .border_1()
                    .border_color(theme.border)
                    .gap(px(16.))
                    .child(
                        h_flex()
                            .items_start()
                            .justify_between()
                            .gap(px(12.))
                            .child(
                                h_flex()
                                    .gap(px(10.))
                                    .items_center()
                                    .child(
                                        img("brands/cc-switch.png").size(px(28.)).flex_shrink_0(),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(16.))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(theme.foreground)
                                            .child(t!("gateway.import_title").to_string()),
                                    ),
                            )
                            .child(
                                Button::new("gateway-import-close")
                                    .ghost()
                                    .xsmall()
                                    .icon(IconName::Close)
                                    .disabled(self.gateway.importing)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.gateway.pane = GatewayPane::List;
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(theme.muted_foreground)
                            .child(t!("gateway.import_hint").to_string()),
                    )
                    .child(
                        v_flex()
                            .gap(px(8.))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(theme.muted_foreground)
                                    .child(t!("gateway.import_client").to_string()),
                            )
                            .child(h_flex().gap(px(8.)).children(clients.iter().map(|app| {
                                let selected = self.gateway.import_app == *app;
                                let app = *app;
                                let brand = match app {
                                    AppKind::Claude => "claude",
                                    AppKind::Codex => "codex",
                                    AppKind::OpenCode => "opencode",
                                    _ => "codex",
                                };
                                let label = match app {
                                    AppKind::Claude => "Claude Code",
                                    AppKind::Codex => "Codex",
                                    AppKind::OpenCode => "OpenCode",
                                    _ => app.display_name(),
                                };
                                v_flex()
                                    .id(SharedString::from(format!(
                                        "gateway-import-app-{}",
                                        app.as_str()
                                    )))
                                    .flex_1()
                                    .gap(px(8.))
                                    .items_center()
                                    .justify_center()
                                    .px(px(10.))
                                    .py(px(12.))
                                    .rounded(px(8.))
                                    .border_1()
                                    .cursor_pointer()
                                    .when(selected, |el| {
                                        el.border_color(Hsla::from(rgb(ASTRLINK_PRIMARY)))
                                            .bg(Hsla::from(rgb(CC_SWITCH_HOVER_BG)))
                                    })
                                    .when(!selected, |el| {
                                        el.border_color(theme.border)
                                            .bg(theme.background)
                                            .hover(|s| s.bg(theme.muted.opacity(0.35)))
                                    })
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.gateway.import_app = app;
                                        cx.notify();
                                    }))
                                    .child(brand_img(brand, dark, px(26.)))
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(if selected {
                                                Hsla::from(rgb(ASTRLINK_PRIMARY))
                                            } else {
                                                theme.foreground
                                            })
                                            .child(label.to_string()),
                                    )
                            }))),
                    )
                    .child(
                        h_flex()
                            .gap(px(16.))
                            .p(px(12.))
                            .rounded(px(8.))
                            .border_1()
                            .border_color(theme.border)
                            .bg(theme.muted.opacity(0.35))
                            .child(
                                h_flex()
                                    .flex_1()
                                    .min_w(px(0.))
                                    .gap(px(10.))
                                    .items_start()
                                    .child(
                                        Icon::new(CustomIcon::Monitor)
                                            .size(px(16.))
                                            .text_color(theme.muted_foreground),
                                    )
                                    .child(
                                        v_flex()
                                            .gap(px(4.))
                                            .min_w(px(0.))
                                            .child(
                                                div()
                                                    .text_size(px(11.))
                                                    .text_color(theme.muted_foreground)
                                                    .child(t!("gateway.api_address").to_string()),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(12.))
                                                    .font_family("ui-monospace")
                                                    .text_color(theme.foreground)
                                                    .child(api_url),
                                            ),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .flex_1()
                                    .min_w(px(0.))
                                    .gap(px(10.))
                                    .items_start()
                                    .child(
                                        Icon::new(CustomIcon::KeyRound)
                                            .size(px(16.))
                                            .text_color(theme.muted_foreground),
                                    )
                                    .child(
                                        v_flex()
                                            .gap(px(4.))
                                            .min_w(px(0.))
                                            .child(
                                                div()
                                                    .text_size(px(11.))
                                                    .text_color(theme.muted_foreground)
                                                    .child(t!("gateway.import_token").to_string()),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(12.))
                                                    .text_color(theme.foreground)
                                                    .overflow_x_hidden()
                                                    .child(token_label),
                                            ),
                                    ),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap(px(12.))
                            .child(
                                v_flex()
                                    .flex_1()
                                    .gap(px(6.))
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(theme.muted_foreground)
                                            .child(t!("gateway.import_name").to_string()),
                                    )
                                    .child(Input::new(&self.gateway.import_name).small()),
                            )
                            .child(
                                v_flex()
                                    .flex_1()
                                    .gap(px(6.))
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(theme.muted_foreground)
                                            .child(
                                                if is_claude {
                                                    t!("gateway.import_default_model")
                                                } else {
                                                    t!("gateway.import_model")
                                                }
                                                .to_string(),
                                            ),
                                    )
                                    .child(
                                        Input::new(&self.gateway.import_model).small().prefix(
                                            Icon::new(IconName::Search)
                                                .size(px(14.))
                                                .text_color(theme.muted_foreground),
                                        ),
                                    ),
                            ),
                    )
                    .when(is_claude, |el| {
                        el.child(
                            h_flex().gap(px(12.)).children(
                                [
                                    (
                                        &self.gateway.import_haiku,
                                        t!("gateway.import_haiku").to_string(),
                                        "gateway-import-haiku",
                                    ),
                                    (
                                        &self.gateway.import_sonnet,
                                        t!("gateway.import_sonnet").to_string(),
                                        "gateway-import-sonnet",
                                    ),
                                    (
                                        &self.gateway.import_opus,
                                        t!("gateway.import_opus").to_string(),
                                        "gateway-import-opus",
                                    ),
                                ]
                                .into_iter()
                                .map(|(input, label, _id)| {
                                    v_flex()
                                        .flex_1()
                                        .gap(px(6.))
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .text_color(theme.muted_foreground)
                                                .child(label),
                                        )
                                        .child(
                                            Input::new(input).small().prefix(
                                                Icon::new(IconName::Search)
                                                    .size(px(14.))
                                                    .text_color(theme.muted_foreground),
                                            ),
                                        )
                                }),
                            ),
                        )
                    })
                    .child(
                        h_flex()
                            .pt(px(12.))
                            .border_t_1()
                            .border_color(theme.border)
                            .justify_end()
                            .gap(px(8.))
                            .child(
                                Button::new("gateway-import-cancel")
                                    .small()
                                    .outline()
                                    .label(t!("common.cancel").to_string())
                                    .disabled(self.gateway.importing)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.gateway.pane = GatewayPane::List;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("gateway-import-confirm")
                                    .small()
                                    .custom(astrlink_primary_btn(cx))
                                    .icon(IconName::ExternalLink)
                                    .label(if self.gateway.importing {
                                        t!("gateway.importing").to_string()
                                    } else {
                                        t!("gateway.import_confirm").to_string()
                                    })
                                    .disabled(self.gateway.importing)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.confirm_gateway_import(window, cx);
                                    })),
                            ),
                    ),
            )
    }

    fn submit_gateway_create(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.gateway.create_name.read(cx).value().trim().to_string();
        if name.is_empty() {
            self.gateway.error = Some(t!("gateway.name_required").to_string());
            cx.notify();
            return;
        }
        if name.chars().count() > 64 {
            self.gateway.error = Some(t!("gateway.name_too_long").to_string());
            cx.notify();
            return;
        }
        self.gateway.creating = true;
        cx.notify();
        match self.workspace.create_access_token(&name) {
            Ok(created) => {
                self.gateway.creating = false;
                self.gateway.pane = GatewayPane::List;
                self.gateway.create_name.update(cx, |s, cx| {
                    s.set_value(String::new(), window, cx);
                });
                self.gateway.items.push(created.token.clone());
                self.gateway.items.sort_by(|a, b| {
                    a.created_at
                        .cmp(&b.created_at)
                        .then_with(|| a.id.cmp(&b.id))
                });
                self.refresh_gateway_token_usage(cx);
                notify_success(
                    t!("gateway.created", name = created.token.name).to_string(),
                    window,
                    cx,
                );
            }
            Err(err) => {
                self.gateway.creating = false;
                self.gateway.error = Some(err.to_string());
            }
        }
        cx.notify();
    }

    fn confirm_gateway_delete(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(token) = self.gateway.pending_delete.clone() else {
            return;
        };
        self.gateway.deleting_id = Some(token.id.clone());
        cx.notify();
        match self.workspace.delete_access_token(&token.id) {
            Ok(()) => {
                self.gateway.items.retain(|t| t.id != token.id);
                self.gateway.usage_by_id.remove(&token.id);
                self.gateway.pending_delete = None;
                self.gateway.deleting_id = None;
                notify_success(
                    t!("gateway.deleted", name = token.name).to_string(),
                    window,
                    cx,
                );
            }
            Err(err) => {
                self.gateway.deleting_id = None;
                self.gateway.error = Some(err.to_string());
            }
        }
        cx.notify();
    }

    fn confirm_gateway_import(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let GatewayPane::Import(token_id) = self.gateway.pane.clone() else {
            return;
        };
        let name = self.gateway.import_name.read(cx).value().trim().to_string();
        let model = self
            .gateway
            .import_model
            .read(cx)
            .value()
            .trim()
            .to_string();
        let haiku = self
            .gateway
            .import_haiku
            .read(cx)
            .value()
            .trim()
            .to_string();
        let sonnet = self
            .gateway
            .import_sonnet
            .read(cx)
            .value()
            .trim()
            .to_string();
        let opus = self.gateway.import_opus.read(cx).value().trim().to_string();
        if name.is_empty() {
            self.gateway.error = Some(t!("gateway.import_name_required").to_string());
            cx.notify();
            return;
        }
        self.gateway.importing = true;
        cx.notify();
        let mut api_url = self.gateway_api_url();
        let app = self.gateway.import_app;
        if matches!(app, AppKind::Codex | AppKind::OpenCode) && !api_url.ends_with("/v1") {
            api_url = format!("{}/v1", api_url.trim_end_matches('/'));
        }
        let token_value = match self.workspace.reveal_access_token(&token_id) {
            Ok(v) => v,
            Err(err) => {
                self.gateway.importing = false;
                self.gateway.error = Some(err.to_string());
                cx.notify();
                return;
            }
        };
        let result = self.import_gateway_token_as_provider(
            app,
            &name,
            &api_url,
            &token_value,
            &model,
            &haiku,
            &sonnet,
            &opus,
            cx,
        );
        self.gateway.importing = false;
        match result {
            Ok(()) => {
                self.gateway.pane = GatewayPane::List;
                notify_success(t!("gateway.import_success").to_string(), window, cx);
            }
            Err(err) => self.gateway.error = Some(err),
        }
        cx.notify();
    }

    fn import_gateway_token_as_provider(
        &mut self,
        app: AppKind,
        name: &str,
        base_url: &str,
        api_key: &str,
        model: &str,
        haiku: &str,
        sonnet: &str,
        opus: &str,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        let model = if model.is_empty() {
            match app {
                AppKind::Codex => DEFAULT_CODEX_MODEL.to_string(),
                AppKind::Claude | AppKind::ClaudeDesktop => DEFAULT_CLAUDE_MODEL.to_string(),
                AppKind::OpenCode => DEFAULT_OPENCODE_MODEL.to_string(),
                _ => String::new(),
            }
        } else {
            model.to_string()
        };

        let form = match app {
            AppKind::Codex => ProviderForm::Codex(CodexForm {
                name: name.to_string(),
                website_url: String::new(),
                kind: CodexKind::ResponsesThirdParty,
                api_key: api_key.to_string(),
                base_url: base_url.to_string(),
                model,
                request_protocol: RequestProtocol::OpenAiResponses.as_str().into(),
                model_mappings: Vec::new(),
            }),
            AppKind::Claude | AppKind::ClaudeDesktop => {
                let mut mappings = Vec::new();
                for (display_name, value) in [("Haiku", haiku), ("Sonnet", sonnet), ("Opus", opus)]
                {
                    if !value.is_empty() {
                        mappings.push(ClaudeModelMapping {
                            display_name: display_name.into(),
                            model: value.to_string(),
                            context_window: None,
                            reasoning_effort: None,
                        });
                    }
                }
                ProviderForm::Claude(ClaudeForm {
                    name: name.to_string(),
                    website_url: String::new(),
                    kind: ClaudeKind::ThirdParty,
                    api_key: api_key.to_string(),
                    base_url: base_url.to_string(),
                    model,
                    request_protocol: RequestProtocol::Anthropic.as_str().into(),
                    model_mappings: mappings,
                    desktop_mode: None,
                })
            }
            AppKind::OpenCode => ProviderForm::OpenCode(OpenCodeForm {
                name: name.to_string(),
                website_url: String::new(),
                kind: OpenCodeKind::ThirdParty,
                api_key: api_key.to_string(),
                base_url: base_url.to_string(),
                model,
                npm: domain::DEFAULT_OPENCODE_NPM.to_string(),
                request_protocol: RequestProtocol::OpenAiChat.as_str().into(),
                model_mappings: Vec::new(),
            }),
            _ => return Err(t!("gateway.import_unsupported_app").to_string()),
        };

        self.workspace
            .save_form(app, None, form)
            .map_err(|e| e.to_string())?;
        self.reload();
        cx.notify();
        Ok(())
    }
}
