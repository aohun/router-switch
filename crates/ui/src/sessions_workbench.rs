//! Wake-style 3-pane sessions workbench (sidebar scope + list + transcript).

use crate::app_view::RouterApp;
use crate::assets::{brand_img, CustomIcon};
use gpui::{
    div, img, list, prelude::FluentBuilder, px, rgb, uniform_list, AbsoluteLength, AnyElement,
    Context, Corner, FontWeight, Hsla, InteractiveElement, IntoElement, ParentElement,
    SharedString, StatefulInteractiveElement, Styled, Window,
};
use gpui_component::{
    button::{Button, ButtonCustomVariant, ButtonVariants as _},
    h_flex,
    input::Input,
    menu::{DropdownMenu, PopupMenuItem},
    notification::Notification,
    scroll::ScrollableElement as _,
    v_flex, ActiveTheme, Icon, IconName, Sizable as _, WindowExt,
};
use rust_i18n::t;
use session::session_index::{
    self, display_agent_name, filter_sessions, OpenTarget, SessionFilter, SessionScope,
    SessionSortKey,
};
use std::collections::HashMap;
use std::path::PathBuf;

fn format_day(ts_ms: Option<i64>) -> String {
    let Some(ts) = ts_ms else {
        return String::new();
    };
    use chrono::TimeZone;
    chrono::Local
        .timestamp_millis_opt(ts)
        .single()
        .map(|dt| {
            if rust_i18n::locale().starts_with("zh") {
                format!("{}月{}日", dt.month(), dt.day())
            } else {
                dt.format("%b %-d").to_string()
            }
        })
        .unwrap_or_default()
}

use chrono::Datelike;

/// Fixed row height for the middle session list (uniform_list virtualization).
const SESSION_ROW_H: f32 = 72.;

/// Wake-style title clip for the middle list (CJK ≈ 2 cells; keep ~36 cells).
fn clip_session_title(raw: &str, max_cells: usize) -> String {
    let one = raw
        .replace(['\r', '\n'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if one.is_empty() {
        return "Untitled".into();
    }
    let mut used = 0usize;
    let mut out = String::new();
    for ch in one.chars() {
        let w = if ch.is_ascii() { 1 } else { 2 };
        if used + w > max_cells {
            out.push('…');
            return out;
        }
        out.push(ch);
        used += w;
    }
    out
}

/// Open In icon: extracted .app png → embedded brand → generic terminal.
fn open_target_icon(
    target: Option<OpenTarget>,
    icons: &HashMap<String, PathBuf>,
    size: AbsoluteLength,
) -> AnyElement {
    if let Some(t) = target {
        if let Some(path) = icons.get(t.id()) {
            return img(path.clone())
                .size(size)
                .flex_shrink_0()
                .into_any_element();
        }
        if let Some(brand) = t.brand_icon() {
            return img(brand).size(size).flex_shrink_0().into_any_element();
        }
    }
    Icon::new(IconName::SquareTerminal)
        .size(size)
        .into_any_element()
}

fn preferred_open_target(app: &RouterApp, targets: &[OpenTarget]) -> Option<OpenTarget> {
    if let Some(pref) = app.preferred_open_target {
        if targets.contains(&pref) {
            return Some(pref);
        }
    }
    targets.first().copied()
}

/// Full Wake-style sessions page.
pub fn render_sessions_workbench(app: &RouterApp, cx: &mut Context<RouterApp>) -> impl IntoElement {
    let theme = cx.theme().clone();
    let dark = theme.is_dark();

    let filter = SessionFilter {
        scope: match app.wake_scope {
            // Favorites / Projects scopes removed from UI — fall back to All.
            SessionScope::Starred | SessionScope::Project => SessionScope::All,
            other => other,
        },
        agent_id: app.wake_agent_filter.clone(),
        project_name: None,
        query: app.sessions_search.read(cx).value().to_string(),
    };
    let visible = filter_sessions(&app.wake_sessions, &filter, app.wake_sort);
    let agent_counts = session_index::agent_counts(&app.wake_sessions);
    let total_n = app.wake_sessions.len();

    let header_title: SharedString = match app.wake_scope {
        SessionScope::Agent => app
            .wake_agent_filter
            .as_deref()
            .map(display_agent_name)
            .unwrap_or_else(|| t!("session_hub.agent").to_string())
            .into(),
        _ => t!("session_hub.all").to_string().into(),
    };

    h_flex()
        .size_full()
        .overflow_hidden()
        // ---- Left scope sidebar ----
        .child(
            v_flex()
                .w(px(220.))
                .h_full()
                .flex_shrink_0()
                .border_r_1()
                .border_color(theme.border)
                .bg(theme.sidebar)
                .p(px(10.))
                .gap(px(8.))
                .child(
                    // Inline filter input (⌘K hint intentionally omitted).
                    // Use default appearance so typed text stays visible.
                    Input::new(&app.sessions_search).cleanable(true),
                )
                .child(scope_row(
                    "wake-scope-all",
                    t!("session_hub.all").to_string(),
                    Some(total_n as i64),
                    matches!(
                        app.wake_scope,
                        SessionScope::All | SessionScope::Starred | SessionScope::Project
                    ) && app.wake_agent_filter.is_none(),
                    cx,
                    |this, _, cx| {
                        this.wake_scope = SessionScope::All;
                        this.wake_agent_filter = None;
                        this.wake_project_filter = None;
                        cx.notify();
                    },
                ))
                // Favorites list intentionally hidden.
                .child(
                    div()
                        .pt(px(8.))
                        .px(px(6.))
                        .text_size(px(11.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.muted_foreground)
                        .child(t!("session_hub.agent").to_string()),
                )
                .child(
                    v_flex()
                        .gap(px(2.))
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scrollbar()
                        .children(agent_counts.into_iter().map(|(id, n)| {
                            let selected = app.wake_scope == SessionScope::Agent
                                && app.wake_agent_filter.as_deref() == Some(id.as_str());
                            let brand = session_index::agent_brand_id(&id);
                            let label = display_agent_name(&id);
                            let id_clone = id.clone();
                            h_flex()
                                .id(SharedString::from(format!("wake-agent-{id}")))
                                .w_full()
                                .items_center()
                                .gap(px(8.))
                                .px(px(8.))
                                .py(px(6.))
                                .rounded(px(8.))
                                .cursor_pointer()
                                .when(selected, |r| r.bg(theme.sidebar_accent))
                                .hover(|s| s.bg(theme.sidebar_accent.opacity(0.6)))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.wake_scope = SessionScope::Agent;
                                    this.wake_agent_filter = Some(id_clone.clone());
                                    this.wake_project_filter = None;
                                    cx.notify();
                                }))
                                .child(brand_img(brand, dark, px(15.)))
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
                                    div()
                                        .text_size(px(11.))
                                        .text_color(theme.muted_foreground)
                                        .child(n.to_string()),
                                )
                        })),
                ),
            // Project stats sidebar removed.
        )
        // ---- Middle session list ----
        .child(
            v_flex()
                .w(px(320.))
                .h_full()
                .flex_shrink_0()
                .border_r_1()
                .border_color(theme.border)
                .bg(theme.background)
                .child(
                    h_flex()
                        .w_full()
                        .h(px(52.))
                        .px(px(14.))
                        .items_center()
                        .justify_between()
                        .border_b_1()
                        .border_color(theme.border)
                        .child(
                            v_flex()
                                .gap(px(2.))
                                .child(
                                    div()
                                        .text_size(px(15.))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(theme.foreground)
                                        .child(header_title.clone()),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(theme.muted_foreground)
                                        .child(format!(
                                            "{} {}",
                                            visible.len(),
                                            t!("session_hub.sessions_unit")
                                        )),
                                ),
                        )
                        .child(
                            h_flex()
                                .gap(px(4.))
                                .child(
                                    Button::new("wake-refresh")
                                        .ghost()
                                        .xsmall()
                                        .icon(CustomIcon::RotateCw)
                                        .tooltip(t!("session_hub.refresh").to_string())
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.refresh_wake_sessions(cx);
                                        })),
                                )
                                .child({
                                    let sort = app.wake_sort;
                                    let sort_tooltip = format!(
                                        "{} · {}",
                                        match sort.key {
                                            SessionSortKey::Updated => {
                                                t!("session_hub.sort_updated")
                                            }
                                            SessionSortKey::Created => {
                                                t!("session_hub.sort_created")
                                            }
                                            SessionSortKey::Messages => {
                                                t!("session_hub.sort_messages")
                                            }
                                        },
                                        if sort.ascending {
                                            t!("session_hub.sort_asc")
                                        } else {
                                            t!("session_hub.sort_desc")
                                        }
                                    );
                                    let entity = cx.entity();
                                    Button::new("wake-sort")
                                        .ghost()
                                        .xsmall()
                                        .icon(CustomIcon::ArrowUpDown)
                                        .tooltip(sort_tooltip)
                                        .dropdown_menu_with_anchor(
                                            Corner::TopRight,
                                            move |menu, _, _| {
                                                let e1 = entity.clone();
                                                let e2 = entity.clone();
                                                let e3 = entity.clone();
                                                let e4 = entity.clone();
                                                let e5 = entity.clone();
                                                menu.min_w(px(180.))
                                                    .item(
                                                        PopupMenuItem::new(
                                                            t!("session_hub.sort_updated")
                                                                .to_string(),
                                                        )
                                                        .checked(sort.key == SessionSortKey::Updated)
                                                        .on_click(move |_, _, cx| {
                                                            e1.update(cx, |this, cx| {
                                                                this.wake_sort.key =
                                                                    SessionSortKey::Updated;
                                                                cx.notify();
                                                            });
                                                        }),
                                                    )
                                                    .item(
                                                        PopupMenuItem::new(
                                                            t!("session_hub.sort_created")
                                                                .to_string(),
                                                        )
                                                        .checked(sort.key == SessionSortKey::Created)
                                                        .on_click(move |_, _, cx| {
                                                            e2.update(cx, |this, cx| {
                                                                this.wake_sort.key =
                                                                    SessionSortKey::Created;
                                                                cx.notify();
                                                            });
                                                        }),
                                                    )
                                                    .item(
                                                        PopupMenuItem::new(
                                                            t!("session_hub.sort_messages")
                                                                .to_string(),
                                                        )
                                                        .checked(
                                                            sort.key == SessionSortKey::Messages,
                                                        )
                                                        .on_click(move |_, _, cx| {
                                                            e3.update(cx, |this, cx| {
                                                                this.wake_sort.key =
                                                                    SessionSortKey::Messages;
                                                                cx.notify();
                                                            });
                                                        }),
                                                    )
                                                    .separator()
                                                    .item(
                                                        PopupMenuItem::new(
                                                            t!("session_hub.sort_desc")
                                                                .to_string(),
                                                        )
                                                        .checked(!sort.ascending)
                                                        .on_click(move |_, _, cx| {
                                                            e4.update(cx, |this, cx| {
                                                                this.wake_sort.ascending = false;
                                                                cx.notify();
                                                            });
                                                        }),
                                                    )
                                                    .item(
                                                        PopupMenuItem::new(
                                                            t!("session_hub.sort_asc")
                                                                .to_string(),
                                                        )
                                                        .checked(sort.ascending)
                                                        .on_click(move |_, _, cx| {
                                                            e5.update(cx, |this, cx| {
                                                                this.wake_sort.ascending = true;
                                                                cx.notify();
                                                            });
                                                        }),
                                                    )
                                            },
                                        )
                                }),
                        ),
                )
                .child(if app.wake_sessions_loading {
                    div()
                        .flex_1()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_color(theme.muted_foreground)
                        .child(t!("session_hub.loading").to_string())
                        .into_any_element()
                } else if visible.is_empty() {
                    div()
                        .flex_1()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_color(theme.muted_foreground)
                        .child(t!("session_hub.empty").to_string())
                        .into_any_element()
                } else {
                    let visible_count = visible.len();
                    let visible_rows = std::sync::Arc::new(visible);
                    div()
                        .id("hub-session-list")
                        .flex_1()
                        .min_h_0()
                        .w_full()
                        .child(
                            uniform_list(
                                "hub-sessions-uniform",
                                visible_count,
                                cx.processor(move |this, range: std::ops::Range<usize>, _window, cx| {
                                    let theme = cx.theme().clone();
                                    let dark = theme.is_dark();
                                    let selected_key = this
                                        .wake_selected
                                        .as_ref()
                                        .map(|s| s.key.clone());
                                    let mut items =
                                        Vec::with_capacity(range.end.saturating_sub(range.start));
                                    for ix in range {
                                        let Some(s) = visible_rows.get(ix) else {
                                            continue;
                                        };
                                        let selected =
                                            selected_key.as_deref() == Some(s.key.as_str());
                                        let key = s.key.clone();
                                        items.push(
                                            h_flex()
                                                .id(SharedString::from(format!(
                                                    "hub-row-{key}"
                                                )))
                                                .w_full()
                                                .h(px(SESSION_ROW_H))
                                                .px(px(12.))
                                                .gap(px(10.))
                                                .items_center()
                                                .cursor_pointer()
                                                .border_b_1()
                                                .border_color(theme.border.opacity(0.5))
                                                .when(selected, |r| {
                                                    r.bg(if dark {
                                                        Hsla::from(rgb(0x303B4C))
                                                    } else {
                                                        Hsla::from(rgb(0xE3EBF6))
                                                    })
                                                })
                                                .hover(|st| st.bg(theme.secondary.opacity(0.7)))
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    if let Some(found) = this
                                                        .wake_sessions
                                                        .iter()
                                                        .find(|x| x.key == key)
                                                        .cloned()
                                                    {
                                                        this.select_wake_session(found, cx);
                                                    }
                                                }))
                                                .child(
                                                    v_flex()
                                                        .flex_1()
                                                        .w_full()
                                                        .min_w_0()
                                                        .gap(px(4.))
                                                        .child(
                                                            // Wake: pre-clip title; don't rely on CSS
                                                            // truncate (collapses to "…" in uniform_list).
                                                            div()
                                                                .w_full()
                                                                .min_w_0()
                                                                .overflow_hidden()
                                                                .text_size(px(13.))
                                                                .font_weight(FontWeight::MEDIUM)
                                                                .text_color(theme.foreground)
                                                                .whitespace_nowrap()
                                                                .child(clip_session_title(
                                                                    &s.title, 36,
                                                                )),
                                                        )
                                                        .child(
                                                            h_flex()
                                                                .gap(px(6.))
                                                                .items_center()
                                                                .min_w_0()
                                                                .child(brand_img(
                                                                    session_index::agent_brand_id(
                                                                        &s.agent_id,
                                                                    ),
                                                                    dark,
                                                                    px(12.),
                                                                ))
                                                                .when_some(
                                                                    s.project_name.clone().filter(|p| {
                                                                        !p.is_empty()
                                                                            && p != "sessions"
                                                                            && p != "main"
                                                                    }),
                                                                    |row, p| {
                                                                        row.child(
                                                                            div()
                                                                                .min_w_0()
                                                                                .overflow_hidden()
                                                                                .text_size(px(11.))
                                                                                .text_color(
                                                                                    theme
                                                                                        .muted_foreground,
                                                                                )
                                                                                .whitespace_nowrap()
                                                                                .child(
                                                                                    clip_session_title(
                                                                                        &p, 28,
                                                                                    ),
                                                                                ),
                                                                        )
                                                                    },
                                                                ),
                                                        ),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(11.))
                                                        .text_color(theme.muted_foreground)
                                                        .flex_shrink_0()
                                                        .child(format_day(
                                                            s.updated_at.or(s.created_at),
                                                        )),
                                                ),
                                        );
                                    }
                                    items
                                }),
                            )
                            .flex_1()
                            .size_full(),
                        )
                        .into_any_element()
                }),
        )
        // ---- Right detail / transcript ----
        .child(render_detail_pane(app, dark, cx))
}

fn scope_row(
    id: &'static str,
    label: String,
    count: Option<i64>,
    selected: bool,
    cx: &mut Context<RouterApp>,
    on_click: impl Fn(&mut RouterApp, &mut Window, &mut Context<RouterApp>) + 'static,
) -> impl IntoElement {
    let theme = cx.theme().clone();
    h_flex()
        .id(id)
        .w_full()
        .items_center()
        .justify_between()
        .px(px(8.))
        .py(px(7.))
        .rounded(px(8.))
        .cursor_pointer()
        .when(selected, |r| r.bg(theme.sidebar_accent))
        .hover(|s| s.bg(theme.sidebar_accent.opacity(0.55)))
        .on_click(cx.listener(move |this, _, window, cx| on_click(this, window, cx)))
        .child(
            div()
                .text_size(px(13.))
                .text_color(theme.foreground)
                .child(label),
        )
        .when_some(count, |row, n| {
            row.child(
                div()
                    .text_size(px(11.))
                    .text_color(theme.muted_foreground)
                    .child(n.to_string()),
            )
        })
}

fn render_detail_pane(
    app: &RouterApp,
    dark: bool,
    cx: &mut Context<RouterApp>,
) -> impl IntoElement {
    let theme = cx.theme().clone();
    let Some(selected) = app.wake_selected.as_ref() else {
        return v_flex()
            .flex_1()
            .h_full()
            .items_center()
            .justify_center()
            .gap(px(8.))
            .bg(theme.background)
            .child(
                div()
                    .text_size(px(15.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.foreground)
                    .child(t!("session_hub.no_selection").to_string()),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(theme.muted_foreground)
                    .child(t!("session_hub.no_selection_hint").to_string()),
            )
            .into_any_element();
    };

    let agent_label = display_agent_name(&selected.agent_id);
    let msg_count = if app.wake_messages_loading {
        "…".to_string()
    } else {
        format!(
            "{} {}",
            app.wake_messages.len(),
            t!("session_hub.messages_unit")
        )
    };

    v_flex()
        .flex_1()
        .h_full()
        .min_w_0()
        .bg(theme.background)
        .child(
            h_flex()
                .w_full()
                .h(px(52.))
                .px(px(16.))
                .items_center()
                .justify_between()
                .border_b_1()
                .border_color(theme.border)
                .child(
                    h_flex()
                        .gap(px(8.))
                        .items_center()
                        .child(brand_img(
                            session_index::agent_brand_id(&selected.agent_id),
                            dark,
                            px(16.),
                        ))
                        .child(
                            div()
                                .text_size(px(13.))
                                .text_color(theme.muted_foreground)
                                .child(agent_label),
                        ),
                )
                .child(
                    h_flex()
                        .gap(px(4.))
                        // Favorite button intentionally hidden.
                        // Pin button intentionally commented out.
                        // .child(
                        //     Button::new("wake-pin")
                        //         .ghost()
                        //         .xsmall()
                        //         .icon(IconName::Heart)
                        //         .tooltip(t!("session_hub.pin").to_string())
                        //         .on_click(cx.listener(|this, _, _, cx| {
                        //             this.toggle_wake_pin(cx);
                        //         })),
                        // )
                        .child({
                            // Wake-style Open In split: left = open preferred, right = chooser.
                            let targets = session_index::open_targets_for(&selected.agent_id);
                            let preferred = preferred_open_target(app, &targets);
                            let entity = cx.entity();
                            let theme = theme.clone();
                            let icons = app.open_target_icons.clone();
                            let menu_icons = icons.clone();
                            h_flex()
                                .h(px(28.))
                                .rounded(theme.radius)
                                .border_1()
                                .border_color(theme.border)
                                .bg(theme.secondary)
                                .overflow_hidden()
                                .child(
                                    div()
                                        .id("sessions-open-in-main")
                                        .h_full()
                                        .px(px(7.))
                                        .flex()
                                        .items_center()
                                        .cursor_pointer()
                                        .hover(|s| s.bg(theme.secondary_hover))
                                        .child(open_target_icon(
                                            preferred,
                                            &icons,
                                            px(16.).into(),
                                        ))
                                        .tooltip({
                                            let label: SharedString = match preferred {
                                                Some(t) => t!(
                                                    "session_hub.open_in_target",
                                                    name = t.label()
                                                )
                                                .to_string()
                                                .into(),
                                                None => {
                                                    t!("session_hub.open_in").to_string().into()
                                                }
                                            };
                                            move |window, cx| {
                                                gpui_component::tooltip::Tooltip::new(label.clone())
                                                    .build(window, cx)
                                            }
                                        })
                                        .on_click({
                                            let entity = entity.clone();
                                            move |_, window, cx| {
                                                if let Some(target) = preferred {
                                                    entity.update(cx, |this, cx| {
                                                        this.open_wake_session(target, window, cx);
                                                    });
                                                } else {
                                                    window.push_notification(
                                                        Notification::warning(
                                                            t!("session_hub.open_unavailable")
                                                                .to_string(),
                                                        ),
                                                        cx,
                                                    );
                                                }
                                            }
                                        }),
                                )
                                .child(
                                    div()
                                        .w(px(1.))
                                        .h_full()
                                        .flex_shrink_0()
                                        .bg(theme.border),
                                )
                                .child(
                                    Button::new("sessions-open-in-more")
                                        .custom(
                                            ButtonCustomVariant::new(cx)
                                                .foreground(theme.muted_foreground)
                                                .hover(theme.secondary_hover)
                                                .active(theme.secondary_active),
                                        )
                                        .rounded(px(0.))
                                        .h(px(26.))
                                        .w(px(22.))
                                        .icon(IconName::ChevronDown)
                                        .tooltip(t!("session_hub.open_in").to_string())
                                        .dropdown_menu_with_anchor(
                                            Corner::TopRight,
                                            move |menu, _, _| {
                                                let mut menu = menu.min_w(px(170.));
                                                if targets.is_empty() {
                                                    menu = menu.item(PopupMenuItem::new(
                                                        t!("session_hub.open_unavailable")
                                                            .to_string(),
                                                    ));
                                                } else {
                                                    for target in targets.clone() {
                                                        let entity = entity.clone();
                                                        let icon_path = menu_icons
                                                            .get(target.id())
                                                            .cloned();
                                                        let brand = target.brand_icon();
                                                        menu = menu.item(
                                                            PopupMenuItem::element(
                                                                move |_, _| {
                                                                    h_flex()
                                                                        .gap(px(8.))
                                                                        .items_center()
                                                                        .child({
                                                                            if let Some(p) =
                                                                                icon_path.clone()
                                                                            {
                                                                                img(p)
                                                                                    .size(px(16.))
                                                                                    .flex_shrink_0()
                                                                                    .into_any_element()
                                                                            } else if let Some(b) =
                                                                                brand
                                                                            {
                                                                                img(b)
                                                                                    .size(px(16.))
                                                                                    .flex_shrink_0()
                                                                                    .into_any_element()
                                                                            } else {
                                                                                Icon::new(
                                                                                    IconName::SquareTerminal,
                                                                                )
                                                                                .size(px(16.))
                                                                                .into_any_element()
                                                                            }
                                                                        })
                                                                        .child(target.label())
                                                                },
                                                            )
                                                            .on_click(move |_, window, cx| {
                                                                entity.update(cx, |this, cx| {
                                                                    this.open_wake_session(
                                                                        target, window, cx,
                                                                    );
                                                                });
                                                            }),
                                                        );
                                                    }
                                                }
                                                menu
                                            },
                                        ),
                                )
                        })
                        .child(
                            Button::new("wake-copy-path")
                                .ghost()
                                .xsmall()
                                .icon(IconName::Copy)
                                .tooltip(t!("session_hub.copy_path").to_string())
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.copy_wake_session_path(window, cx);
                                })),
                        )
                        .child(
                            Button::new("wake-reveal")
                                .ghost()
                                .xsmall()
                                .icon(IconName::Folder)
                                .tooltip(t!("session_hub.reveal").to_string())
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.reveal_wake_session(window, cx);
                                })),
                        )
                        .child(
                            Button::new("wake-export")
                                .ghost()
                                .xsmall()
                                .icon(IconName::File)
                                .tooltip(t!("session_hub.export").to_string())
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.export_wake_session(window, cx);
                                })),
                        ),
                ),
        )
        .child(
            v_flex()
                .w_full()
                .px(px(20.))
                .py(px(14.))
                .gap(px(6.))
                .border_b_1()
                .border_color(theme.border)
                .child(
                    div()
                        .text_size(px(22.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.foreground)
                        .child(selected.title.clone()),
                )
                .child(
                    h_flex()
                        .gap(px(12.))
                        .flex_wrap()
                        .text_size(px(12.))
                        .text_color(theme.muted_foreground)
                        .child(msg_count)
                        .when_some(selected.project_name.clone(), |row, p| {
                            row.child(p)
                        })
                        .child(format!(
                            "{} {}",
                            t!("session_hub.created"),
                            format_day(selected.created_at)
                        ))
                        .child(format!(
                            "{} {}",
                            t!("session_hub.updated"),
                            format_day(selected.updated_at)
                        ))
                        .when_some(selected.source_path.clone(), |row, p| {
                            row.child(
                                div().truncate().max_w(px(280.)).child(p),
                            )
                        }),
                ),
        )
        .child(if app.wake_messages_loading {
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme.muted_foreground)
                .child(t!("session_hub.loading_messages").to_string())
                .into_any_element()
        } else if app.wake_messages.is_empty() {
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme.muted_foreground)
                .child(t!("session_hub.empty").to_string())
                .into_any_element()
        } else {
            // Wake-style variable-height virtual list for transcript.
            let entity = cx.entity().downgrade();
            let msg_list = app.wake_msg_list.clone();
            div()
                .flex_1()
                .min_h_0()
                .w_full()
                .bg(theme.background)
                .relative()
                .child(
                    list(msg_list.clone(), move |ix, _window, cx| {
                        entity
                            .upgrade()
                            .map(|e| {
                                e.update(cx, |this, cx| this.render_wake_msg_row(ix, cx))
                            })
                            .unwrap_or_else(|| div().into_any_element())
                    })
                    .size_full(),
                )
                .vertical_scrollbar(&msg_list)
                .into_any_element()
        })
        .into_any_element()
}
