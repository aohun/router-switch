//! AstrLink-aligned Smart Routing settings page (UI + autosave).
//! Runtime gateway apply is deferred.

use std::collections::BTreeSet;

use domain::{
    default_failure_policy, ensure_builtin_redirects, is_builtin_model_redirect,
    model_redirect_issues, validate_routing_settings, FailoverStrategy, FailureAction,
    ModelRedirect, ModelRedirectIssue, RoutingSettings, MAX_MODEL_REDIRECTS,
    MAX_REDIRECT_MODEL_LEN,
};
use gpui::{
    div, prelude::FluentBuilder, px, App, AppContext, Context, Entity, FontWeight,
    InteractiveElement, IntoElement, MouseButton, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, Subscription, Timer, Window,
};
use gpui_component::{
    button::{Button, ButtonVariants},
    checkbox::Checkbox,
    h_flex,
    input::{Input, InputEvent, InputState},
    notification::Notification,
    scroll::ScrollableElement as _,
    select::{Select, SelectEvent, SelectItem, SelectState},
    switch::Switch,
    v_flex, ActiveTheme, Disableable, Icon, IconName, Sizable, StyledExt, WindowExt,
};
use rust_i18n::t;

use crate::app_view::RouterApp;
use crate::assets::CustomIcon;

const AUTOSAVE_MS: u64 = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SmartRoutingTab {
    #[default]
    Redirects,
    Recovery,
    Rules,
    Session,
    Identity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RoutingSaveStatus {
    #[default]
    Idle,
    Dirty,
    Saving,
    Saved,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailoverStrategySelectItem {
    pub strategy: FailoverStrategy,
    pub label: String,
}

impl SelectItem for FailoverStrategySelectItem {
    type Value = FailoverStrategy;
    fn title(&self) -> SharedString {
        self.label.clone().into()
    }
    fn value(&self) -> &Self::Value {
        &self.strategy
    }
}

fn failover_strategy_select_items() -> Vec<FailoverStrategySelectItem> {
    // AstrLink RecoveryOrderControls: retry_first + failover_only only.
    vec![
        FailoverStrategySelectItem {
            strategy: FailoverStrategy::RetryFirst,
            label: t!("smart_routing.strategy_retry_first").to_string(),
        },
        FailoverStrategySelectItem {
            strategy: FailoverStrategy::FailoverOnly,
            label: t!("smart_routing.strategy_failover_only").to_string(),
        },
    ]
}

pub struct SmartRoutingState {
    pub draft: RoutingSettings,
    pub baseline: String,
    pub tab: SmartRoutingTab,
    pub save_status: RoutingSaveStatus,
    pub save_error: Option<String>,
    pub editing_redirect: bool,
    pub model_options: Vec<String>,
    pub autosave_token: u64,
    pub show_redirect_issues: bool,
    /// Parallel to `draft.model_redirects`.
    pub redirect_from: Vec<Entity<InputState>>,
    pub redirect_to: Vec<Entity<InputState>>,
    /// Keep InputEvent subscriptions alive for redirect combobox fields.
    pub redirect_input_subs: Vec<Subscription>,
    /// Open model dropdown: `(row_index, is_from)`. Anchored under the input like AstrLink.
    pub redirect_dropdown: Option<(usize, bool)>,
    /// Filter text for the open dropdown. Only applied when `redirect_filter_active`.
    pub redirect_filter_query: String,
    /// AstrLink: opening shows every option; only typing after open filters the list.
    pub redirect_filter_active: bool,
    /// Input value captured when the dropdown opened. Change events that still equal
    /// this baseline are ignored for filtering (click/focus must not search by value).
    pub redirect_filter_baseline: String,
    /// After choose / Enter / chevron-close, ignore the next Focus-driven reopen.
    pub redirect_block_open: bool,
    pub strategy_select: Entity<SelectState<Vec<FailoverStrategySelectItem>>>,
    pub strategy_select_sub: Option<Subscription>,
    pub max_retries_input: Entity<InputState>,
    pub initial_delay_input: Entity<InputState>,
    pub max_delay_input: Entity<InputState>,
    pub max_attempts_input: Entity<InputState>,
    pub ttl_minutes_input: Entity<InputState>,
    /// AstrLink FailureRulesEditor: add HTTP status code field.
    pub http_status_input: Entity<InputState>,
    pub http_status_input_sub: Option<Subscription>,
    /// Keep InputEvent subscriptions alive for numeric recovery / session fields.
    pub number_input_subs: Vec<Subscription>,
}

impl SmartRoutingState {
    pub fn new(window: &mut Window, cx: &mut Context<RouterApp>) -> Self {
        let max_retries_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t!("smart_routing.max_retries").to_string())
        });
        let initial_delay_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t!("smart_routing.initial_delay").to_string())
        });
        let max_delay_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t!("smart_routing.max_delay").to_string())
        });
        let max_attempts_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t!("smart_routing.max_attempts").to_string())
        });
        let ttl_minutes_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t!("smart_routing.ttl_minutes").to_string())
        });
        let http_status_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("smart_routing.add_status_placeholder").to_string())
        });
        let strategy_select = cx.new(|cx| {
            SelectState::new(
                failover_strategy_select_items(),
                Some(gpui_component::IndexPath::default().row(1)),
                window,
                cx,
            )
        });
        Self {
            draft: RoutingSettings::default(),
            baseline: String::new(),
            tab: SmartRoutingTab::Redirects,
            save_status: RoutingSaveStatus::Idle,
            save_error: None,
            editing_redirect: false,
            model_options: Vec::new(),
            autosave_token: 0,
            show_redirect_issues: false,
            redirect_from: Vec::new(),
            redirect_to: Vec::new(),
            redirect_input_subs: Vec::new(),
            redirect_dropdown: None,
            redirect_filter_query: String::new(),
            redirect_filter_active: false,
            redirect_filter_baseline: String::new(),
            redirect_block_open: false,
            strategy_select,
            strategy_select_sub: None,
            max_retries_input,
            initial_delay_input,
            max_delay_input,
            max_attempts_input,
            ttl_minutes_input,
            http_status_input,
            http_status_input_sub: None,
            number_input_subs: Vec::new(),
        }
    }
}

const REDIRECT_MENU_MAX: usize = 100;

fn redirect_issue_message(issue: ModelRedirectIssue) -> String {
    match issue {
        ModelRedirectIssue::EmptyFrom => t!("smart_routing.issue_empty_from").to_string(),
        ModelRedirectIssue::EmptyTo => t!("smart_routing.issue_empty_to").to_string(),
        ModelRedirectIssue::TooLong => {
            t!("smart_routing.issue_too_long", max = MAX_REDIRECT_MODEL_LEN).to_string()
        }
        ModelRedirectIssue::ControlCharacter => {
            t!("smart_routing.issue_control_character").to_string()
        }
        ModelRedirectIssue::Whitespace => t!("smart_routing.issue_whitespace").to_string(),
        ModelRedirectIssue::SameModel => t!("smart_routing.issue_same_model").to_string(),
        ModelRedirectIssue::AutoTarget => {
            t!("smart_routing.issue_auto_target", model = "astrlink/auto").to_string()
        }
        ModelRedirectIssue::DuplicateFrom => t!("smart_routing.issue_duplicate_from").to_string(),
        ModelRedirectIssue::ChainedTarget => t!("smart_routing.issue_chained_target").to_string(),
    }
}

fn http_status_label(code: &str) -> String {
    match code {
        "401" => t!("smart_routing.status_401").to_string(),
        "403" => t!("smart_routing.status_403").to_string(),
        "408" => t!("smart_routing.status_408").to_string(),
        "429" => t!("smart_routing.status_429").to_string(),
        "500" => t!("smart_routing.status_500").to_string(),
        "502" => t!("smart_routing.status_502").to_string(),
        "503" => t!("smart_routing.status_503").to_string(),
        "504" => t!("smart_routing.status_504").to_string(),
        "529" => t!("smart_routing.status_529").to_string(),
        _ => String::new(),
    }
}

fn parse_http_status_code(raw: &str) -> Option<String> {
    let code = raw.trim();
    if code.len() == 3
        && code.chars().all(|c| c.is_ascii_digit())
        && (code.starts_with('4') || code.starts_with('5'))
    {
        Some(code.to_string())
    } else {
        None
    }
}

impl RouterApp {
    pub(crate) fn load_smart_routing(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut settings = self
            .workspace
            .routing_settings()
            .unwrap_or_else(|_| RoutingSettings::default());
        ensure_builtin_redirects(&mut settings);
        self.smart_routing.model_options = self.collect_routing_model_options();
        self.ensure_routing_number_input_subs(window, cx);
        self.ensure_routing_strategy_select_sub(window, cx);
        self.ensure_routing_http_status_input_sub(window, cx);
        self.sync_routing_inputs_from_settings(&settings, window, cx);
        self.smart_routing.baseline =
            serde_json::to_string(&settings).unwrap_or_else(|_| "{}".into());
        self.smart_routing.draft = settings;
        self.smart_routing.save_status = RoutingSaveStatus::Idle;
        self.smart_routing.save_error = None;
        self.smart_routing.show_redirect_issues = false;
        cx.notify();
    }

    fn ensure_routing_number_input_subs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.smart_routing.number_input_subs.is_empty() {
            return;
        }
        let fields = [
            self.smart_routing.max_attempts_input.clone(),
            self.smart_routing.max_retries_input.clone(),
            self.smart_routing.initial_delay_input.clone(),
            self.smart_routing.max_delay_input.clone(),
            self.smart_routing.ttl_minutes_input.clone(),
        ];
        for input in fields {
            let view = cx.entity().downgrade();
            let sub = window.subscribe(&input, cx, move |_, event: &InputEvent, window, cx| {
                if !matches!(
                    event,
                    InputEvent::Change | InputEvent::Blur | InputEvent::PressEnter { .. }
                ) {
                    return;
                }
                if let Some(entity) = view.upgrade() {
                    entity.update(cx, |this, cx| {
                        this.mark_routing_dirty(window, cx);
                    });
                }
            });
            self.smart_routing.number_input_subs.push(sub);
        }
    }

    fn ensure_routing_strategy_select_sub(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.smart_routing.strategy_select_sub.is_some() {
            return;
        }
        let select = self.smart_routing.strategy_select.clone();
        let view = cx.entity().downgrade();
        let sub = window.subscribe(
            &select,
            cx,
            move |_, event: &SelectEvent<Vec<FailoverStrategySelectItem>>, window, cx| {
                if let SelectEvent::Confirm(Some(strategy)) = event {
                    if let Some(entity) = view.upgrade() {
                        let strategy = *strategy;
                        entity.update(cx, |this, cx| {
                            this.smart_routing.draft.strategy = strategy;
                            this.mark_routing_dirty(window, cx);
                        });
                    }
                }
            },
        );
        self.smart_routing.strategy_select_sub = Some(sub);
    }

    fn ensure_routing_http_status_input_sub(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.smart_routing.http_status_input_sub.is_some() {
            return;
        }
        let input = self.smart_routing.http_status_input.clone();
        let view = cx.entity().downgrade();
        let sub = window.subscribe(
            &input,
            cx,
            move |_, event: &InputEvent, window, cx| match event {
                InputEvent::PressEnter { .. } => {
                    if let Some(entity) = view.upgrade() {
                        entity.update(cx, |this, cx| {
                            this.try_add_routing_http_status_rule(window, cx);
                        });
                    }
                }
                InputEvent::Change => {
                    if let Some(entity) = view.upgrade() {
                        entity.update(cx, |_this, cx| {
                            cx.notify();
                        });
                    }
                }
                _ => {}
            },
        );
        self.smart_routing.http_status_input_sub = Some(sub);
    }

    fn try_add_routing_http_status_rule(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let raw = self
            .smart_routing
            .http_status_input
            .read(cx)
            .value()
            .to_string();
        let Some(code) = parse_http_status_code(&raw) else {
            return;
        };
        if self
            .smart_routing
            .draft
            .default_failure_policy
            .http_status
            .contains_key(&code)
        {
            return;
        }
        self.smart_routing
            .draft
            .default_failure_policy
            .http_status
            .insert(code, FailureAction::RetryAndFailover);
        self.smart_routing
            .http_status_input
            .update(cx, |input, cx| {
                input.set_value("", window, cx);
            });
        self.mark_routing_dirty(window, cx);
    }

    fn reset_routing_rules_section(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let defaults = default_failure_policy();
        let policy = &mut self.smart_routing.draft.default_failure_policy;
        policy.network_error = defaults.network_error;
        policy.response_timeout = defaults.response_timeout;
        policy.http_status = defaults.http_status;
        self.mark_routing_dirty(window, cx);
    }

    fn collect_routing_model_options(&self) -> Vec<String> {
        let mut set = BTreeSet::new();
        if let Ok(items) = self.workspace.list_api_providers() {
            for p in items.into_iter().filter(|p| p.enabled) {
                for m in p.models {
                    if !m.trim().is_empty() {
                        set.insert(m);
                    }
                }
            }
        }
        set.into_iter().collect()
    }

    fn sync_routing_inputs_from_settings(
        &mut self,
        settings: &RoutingSettings,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let policy = &settings.default_failure_policy;
        self.smart_routing
            .max_retries_input
            .update(cx, |input, cx| {
                input.set_value(policy.max_retries.to_string(), window, cx);
            });
        self.smart_routing
            .initial_delay_input
            .update(cx, |input, cx| {
                input.set_value(policy.initial_delay_ms.to_string(), window, cx);
            });
        self.smart_routing.max_delay_input.update(cx, |input, cx| {
            input.set_value(policy.max_delay_ms.to_string(), window, cx);
        });
        self.smart_routing
            .max_attempts_input
            .update(cx, |input, cx| {
                input.set_value(settings.max_attempts.to_string(), window, cx);
            });
        let select_strategy = match settings.strategy {
            FailoverStrategy::FailoverOnly => FailoverStrategy::FailoverOnly,
            FailoverStrategy::RetryFirst | FailoverStrategy::FailoverFirst => {
                FailoverStrategy::RetryFirst
            }
        };
        self.smart_routing.strategy_select.update(cx, |select, cx| {
            select.set_items(failover_strategy_select_items(), window, cx);
            select.set_selected_value(&select_strategy, window, cx);
        });
        let minutes = (settings.channel_stickiness.ttl_seconds / 60).max(1);
        self.smart_routing
            .ttl_minutes_input
            .update(cx, |input, cx| {
                input.set_value(minutes.to_string(), window, cx);
            });

        self.smart_routing.redirect_input_subs.clear();
        self.close_redirect_model_dropdown();
        self.smart_routing.redirect_from.clear();
        self.smart_routing.redirect_to.clear();
        for (i, r) in settings.model_redirects.iter().enumerate() {
            let from = self.new_redirect_model_input(
                i,
                true,
                &r.from,
                t!("smart_routing.from_placeholder").to_string(),
                window,
                cx,
            );
            let to = self.new_redirect_model_input(
                i,
                false,
                &r.to,
                t!("smart_routing.to_placeholder").to_string(),
                window,
                cx,
            );
            self.smart_routing.redirect_from.push(from);
            self.smart_routing.redirect_to.push(to);
        }
    }

    fn new_redirect_model_input(
        &mut self,
        index: usize,
        is_from: bool,
        value: &str,
        placeholder: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<InputState> {
        let placeholder = placeholder.into();
        let initial = value.to_string();
        let input = cx.new(|cx| {
            let mut s = InputState::new(window, cx).placeholder(placeholder);
            if !initial.is_empty() {
                s.set_value(initial, window, cx);
            }
            s
        });
        let view = cx.entity().downgrade();
        let input_for_events = input.clone();
        let sub = window.subscribe(&input, cx, move |_, event: &InputEvent, window, cx| {
            match event {
                InputEvent::Focus => {
                    if let Some(entity) = view.upgrade() {
                        entity.update(cx, |this, cx| {
                            let key = (index, is_from);
                            if this.smart_routing.redirect_dropdown == Some(key) {
                                return;
                            }
                            // Choosing a row closes the menu then may re-focus the input;
                            // consume that Focus so the list stays closed (AstrLink choose).
                            if this.smart_routing.redirect_block_open {
                                this.smart_routing.redirect_block_open = false;
                                this.smart_routing.editing_redirect = true;
                                cx.notify();
                                return;
                            }
                            let baseline = input_for_events.read(cx).value().to_string();
                            this.open_redirect_model_dropdown(index, is_from, &baseline, cx);
                        });
                    }
                }
                InputEvent::Change => {
                    let typed = input_for_events.read(cx).value().to_string();
                    if let Some(entity) = view.upgrade() {
                        entity.update(cx, |this, cx| {
                            let key = (index, is_from);
                            // Never open from Change — pick/set_value would reopen the menu,
                            // and click echoes must not become a fuzzy query (AstrLink).
                            if this.smart_routing.redirect_dropdown == Some(key) {
                                let baseline = &this.smart_routing.redirect_filter_baseline;
                                if this.smart_routing.redirect_filter_active || typed != *baseline {
                                    this.smart_routing.redirect_filter_query = typed;
                                    this.smart_routing.redirect_filter_active = true;
                                }
                            }
                            this.smart_routing.editing_redirect = true;
                            this.pull_routing_draft_from_inputs(cx);
                            this.smart_routing.save_status = RoutingSaveStatus::Dirty;
                            cx.notify();
                        });
                    }
                }
                InputEvent::PressEnter { .. } => {
                    if let Some(entity) = view.upgrade() {
                        entity.update(cx, |this, cx| {
                            this.close_redirect_model_dropdown();
                            this.smart_routing.editing_redirect = false;
                            this.mark_routing_dirty(window, cx);
                        });
                    }
                }
                InputEvent::Blur => {
                    if let Some(entity) = view.upgrade() {
                        entity.update(cx, |this, cx| {
                            // Keep list open across option mouse_down; close on Enter / pick.
                            this.smart_routing.editing_redirect = false;
                            this.mark_routing_dirty(window, cx);
                        });
                    }
                }
            }
        });
        self.smart_routing.redirect_input_subs.push(sub);
        input
    }

    fn open_redirect_model_dropdown(
        &mut self,
        index: usize,
        is_from: bool,
        baseline: &str,
        cx: &mut Context<Self>,
    ) {
        self.smart_routing.redirect_block_open = false;
        self.smart_routing.redirect_dropdown = Some((index, is_from));
        self.smart_routing.redirect_filter_query.clear();
        self.smart_routing.redirect_filter_active = false;
        self.smart_routing.redirect_filter_baseline = baseline.to_string();
        self.smart_routing.editing_redirect = true;
        cx.notify();
    }

    fn close_redirect_model_dropdown(&mut self) {
        let was_open = self.smart_routing.redirect_dropdown.is_some();
        self.smart_routing.redirect_dropdown = None;
        self.smart_routing.redirect_filter_query.clear();
        self.smart_routing.redirect_filter_active = false;
        self.smart_routing.redirect_filter_baseline.clear();
        if was_open {
            self.smart_routing.redirect_block_open = true;
        }
    }

    /// Pull number / redirect inputs into `draft` before validate/save.
    fn pull_routing_draft_from_inputs(&mut self, cx: &mut Context<Self>) {
        let parse_u32 = |entity: &Entity<InputState>, cx: &mut Context<Self>| -> Option<u32> {
            entity.read(cx).value().trim().parse().ok()
        };
        if let Some(v) = parse_u32(&self.smart_routing.max_retries_input, cx) {
            self.smart_routing.draft.default_failure_policy.max_retries = v;
        }
        if let Some(v) = parse_u32(&self.smart_routing.initial_delay_input, cx) {
            self.smart_routing
                .draft
                .default_failure_policy
                .initial_delay_ms = v;
        }
        if let Some(v) = parse_u32(&self.smart_routing.max_delay_input, cx) {
            self.smart_routing.draft.default_failure_policy.max_delay_ms = v;
        }
        if let Some(v) = parse_u32(&self.smart_routing.max_attempts_input, cx) {
            self.smart_routing.draft.max_attempts = v;
        }
        if let Some(mins) = parse_u32(&self.smart_routing.ttl_minutes_input, cx) {
            self.smart_routing.draft.channel_stickiness.ttl_seconds =
                mins.saturating_mul(60).clamp(60, 86_400);
        }
        let n = self
            .smart_routing
            .draft
            .model_redirects
            .len()
            .min(self.smart_routing.redirect_from.len())
            .min(self.smart_routing.redirect_to.len());
        for i in 0..n {
            // Built-in source id is locked; never overwrite it from a stale input.
            if !is_builtin_model_redirect(&self.smart_routing.draft.model_redirects, i) {
                let from = self.smart_routing.redirect_from[i]
                    .read(cx)
                    .value()
                    .to_string();
                self.smart_routing.draft.model_redirects[i].from = from;
            }
            let to = self.smart_routing.redirect_to[i]
                .read(cx)
                .value()
                .to_string();
            self.smart_routing.draft.model_redirects[i].to = to;
        }
    }

    pub(crate) fn mark_routing_dirty(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.pull_routing_draft_from_inputs(cx);
        let json = serde_json::to_string(&self.smart_routing.draft).unwrap_or_default();
        if json == self.smart_routing.baseline {
            self.smart_routing.save_status = RoutingSaveStatus::Idle;
            cx.notify();
            return;
        }
        self.smart_routing.save_status = RoutingSaveStatus::Dirty;
        self.smart_routing.autosave_token = self.smart_routing.autosave_token.wrapping_add(1);
        let token = self.smart_routing.autosave_token;
        cx.notify();
        if self.smart_routing.editing_redirect {
            return;
        }
        let view = cx.entity().downgrade();
        window
            .spawn(cx, move |cx: &mut gpui::AsyncWindowContext| {
                let mut cx = cx.clone();
                async move {
                    Timer::after(std::time::Duration::from_millis(AUTOSAVE_MS)).await;
                    let _ = cx.update(|window: &mut Window, cx: &mut App| {
                        let _ = view.update(cx, |this, cx| {
                            if this.smart_routing.autosave_token != token {
                                return;
                            }
                            if this.smart_routing.editing_redirect {
                                return;
                            }
                            this.persist_smart_routing(window, cx);
                        });
                    });
                }
            })
            .detach();
    }

    pub(crate) fn persist_smart_routing(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.pull_routing_draft_from_inputs(cx);
        if let Err(err) = validate_routing_settings(&self.smart_routing.draft) {
            // AstrLink: redirect field errors stay on the row; status stays 「未保存」.
            if err == "invalid model_redirects" {
                self.smart_routing.save_status = RoutingSaveStatus::Dirty;
                self.smart_routing.save_error = None;
                self.smart_routing.show_redirect_issues = true;
            } else {
                self.smart_routing.save_status = RoutingSaveStatus::Error;
                self.smart_routing.save_error = Some(err);
                self.smart_routing.show_redirect_issues = true;
            }
            cx.notify();
            return;
        }
        self.smart_routing.save_status = RoutingSaveStatus::Saving;
        self.smart_routing.save_error = None;
        cx.notify();
        match self
            .workspace
            .save_routing_settings(&self.smart_routing.draft)
        {
            Ok(()) => {
                self.smart_routing.baseline =
                    serde_json::to_string(&self.smart_routing.draft).unwrap_or_default();
                self.smart_routing.save_status = RoutingSaveStatus::Saved;
                self.smart_routing.show_redirect_issues = false;
                cx.notify();
            }
            Err(err) => {
                self.smart_routing.save_status = RoutingSaveStatus::Error;
                self.smart_routing.save_error = Some(err.to_string());
                window.push_notification(Notification::warning(err.to_string()), cx);
                cx.notify();
            }
        }
    }

    pub(crate) fn add_model_redirect(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.smart_routing.draft.model_redirects.len() >= MAX_MODEL_REDIRECTS {
            return;
        }
        self.smart_routing.model_options = self.collect_routing_model_options();
        let index = self.smart_routing.draft.model_redirects.len();
        self.smart_routing
            .draft
            .model_redirects
            .push(ModelRedirect {
                from: String::new(),
                to: String::new(),
                enabled: true,
            });
        let from = self.new_redirect_model_input(
            index,
            true,
            "",
            t!("smart_routing.from_placeholder").to_string(),
            window,
            cx,
        );
        let to = self.new_redirect_model_input(
            index,
            false,
            "",
            t!("smart_routing.to_placeholder").to_string(),
            window,
            cx,
        );
        self.smart_routing.redirect_from.push(from);
        self.smart_routing.redirect_to.push(to);
        self.smart_routing.redirect_dropdown = Some((index, true));
        self.smart_routing.editing_redirect = true;
        self.smart_routing.save_status = RoutingSaveStatus::Dirty;
        cx.notify();
    }

    pub(crate) fn remove_model_redirect(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if index >= self.smart_routing.draft.model_redirects.len() {
            return;
        }
        self.pull_routing_draft_from_inputs(cx);
        if is_builtin_model_redirect(&self.smart_routing.draft.model_redirects, index) {
            // Builtin row: disable instead of remove.
            self.smart_routing.draft.model_redirects[index].enabled = false;
        } else {
            self.smart_routing.draft.model_redirects.remove(index);
            // Recreate inputs so subscription indices stay aligned with rows.
            let snapshot = self.smart_routing.draft.clone();
            self.sync_routing_inputs_from_settings(&snapshot, window, cx);
        }
        self.smart_routing.editing_redirect = false;
        self.mark_routing_dirty(window, cx);
    }

    pub(crate) fn render_smart_routing_page(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let status = match self.smart_routing.save_status {
            RoutingSaveStatus::Idle => t!("smart_routing.autosave_idle").to_string(),
            RoutingSaveStatus::Dirty => t!("smart_routing.autosave_unsaved").to_string(),
            RoutingSaveStatus::Saving => t!("smart_routing.autosave_saving").to_string(),
            RoutingSaveStatus::Saved => t!("smart_routing.autosave_saved").to_string(),
            RoutingSaveStatus::Error => self
                .smart_routing
                .save_error
                .clone()
                .unwrap_or_else(|| t!("smart_routing.autosave_error").to_string()),
        };
        let status_color = match self.smart_routing.save_status {
            RoutingSaveStatus::Error => theme.danger,
            RoutingSaveStatus::Saved => theme.success,
            _ => theme.muted_foreground,
        };

        v_flex()
            .size_full()
            .bg(theme.background)
            // AstrLink PageHeader: title + bottom border
            .child(
                h_flex()
                    .w_full()
                    .px(px(24.))
                    .pt(px(20.))
                    .pb(px(12.))
                    .items_center()
                    .border_b_1()
                    .border_color(theme.border)
                    .child(
                        div()
                            .text_size(px(22.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.foreground)
                            .child(t!("smart_routing.title").to_string()),
                    ),
            )
            // AstrLink TabsList: muted pill track + autosave on the right
            .child(
                h_flex()
                    .w_full()
                    .px(px(24.))
                    .pt(px(12.))
                    .pb(px(12.))
                    .items_center()
                    .justify_between()
                    .gap(px(12.))
                    .child(
                        h_flex()
                            .items_center()
                            .rounded(px(8.))
                            .p(px(3.))
                            .bg(theme.secondary)
                            .gap(px(0.))
                            .child(self.routing_pill_tab(
                                "sr-tab-redirects",
                                SmartRoutingTab::Redirects,
                                t!("smart_routing.tab_redirects").to_string(),
                                cx,
                            ))
                            .child(self.routing_tab_divider(cx))
                            .child(self.routing_pill_tab(
                                "sr-tab-recovery",
                                SmartRoutingTab::Recovery,
                                t!("smart_routing.tab_recovery").to_string(),
                                cx,
                            ))
                            .child(self.routing_tab_divider(cx))
                            .child(self.routing_pill_tab(
                                "sr-tab-rules",
                                SmartRoutingTab::Rules,
                                t!("smart_routing.tab_rules").to_string(),
                                cx,
                            ))
                            .child(self.routing_tab_divider(cx))
                            .child(self.routing_pill_tab(
                                "sr-tab-session",
                                SmartRoutingTab::Session,
                                t!("smart_routing.tab_session").to_string(),
                                cx,
                            ))
                            .child(self.routing_tab_divider(cx))
                            .child(self.routing_pill_tab(
                                "sr-tab-identity",
                                SmartRoutingTab::Identity,
                                t!("smart_routing.tab_identity").to_string(),
                                cx,
                            )),
                    )
                    .child(
                        div()
                            .flex_shrink_0()
                            .text_size(px(12.))
                            .text_color(status_color)
                            .child(status),
                    ),
            )
            .child(
                // Match gateway: vertical scrollbar only; content height = last panel
                // (overflow_scroll allowed unbounded empty scroll past the bottom).
                v_flex()
                    .id("smart-routing-body")
                    .flex_1()
                    .min_h(px(0.))
                    .w_full()
                    .px(px(24.))
                    .pb(px(24.))
                    .overflow_y_scrollbar()
                    .child(match self.smart_routing.tab {
                        SmartRoutingTab::Redirects => {
                            self.render_routing_redirects_tab(cx).into_any_element()
                        }
                        SmartRoutingTab::Recovery => {
                            self.render_routing_recovery_tab(cx).into_any_element()
                        }
                        SmartRoutingTab::Rules => {
                            self.render_routing_rules_tab(cx).into_any_element()
                        }
                        SmartRoutingTab::Session => {
                            self.render_routing_session_tab(cx).into_any_element()
                        }
                        SmartRoutingTab::Identity => {
                            self.render_routing_identity_tab(cx).into_any_element()
                        }
                    }),
            )
    }

    fn routing_tab_divider(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        div()
            .w(px(1.))
            .h(px(14.))
            .mx(px(2.))
            .bg(theme.border.opacity(0.8))
    }

    fn routing_pill_tab(
        &self,
        id: &'static str,
        tab: SmartRoutingTab,
        label: String,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme().clone();
        let selected = self.smart_routing.tab == tab;
        h_flex()
            .id(SharedString::from(id))
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
            .on_click(cx.listener(move |this, _, window, cx| {
                // Commit in-progress redirect field edits when leaving the tab.
                if this.smart_routing.tab != tab {
                    this.smart_routing.editing_redirect = false;
                    this.pull_routing_draft_from_inputs(cx);
                    let _ = this.persist_if_dirty(window, cx);
                }
                this.smart_routing.tab = tab;
                cx.notify();
            }))
            .child(label)
    }

    fn persist_if_dirty(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.pull_routing_draft_from_inputs(cx);
        let json = serde_json::to_string(&self.smart_routing.draft).unwrap_or_default();
        if json != self.smart_routing.baseline {
            self.persist_smart_routing(window, cx);
        }
    }

    fn render_routing_redirects_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let count = self.smart_routing.draft.model_redirects.len();
        let issues = model_redirect_issues(&self.smart_routing.draft.model_redirects);
        let show_issues = self.smart_routing.show_redirect_issues;

        v_flex()
            .w_full()
            .gap(px(12.))
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .gap(px(12.))
                    .child(
                        h_flex()
                            .items_baseline()
                            .gap(px(8.))
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(t!("smart_routing.redirects_title").to_string()),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(theme.muted_foreground)
                                    .child(format!(
                                        "{} {}",
                                        count,
                                        t!("smart_routing.redirects_unit")
                                    )),
                            ),
                    )
                    .child(
                        Button::new("sr-add-redirect")
                            .outline()
                            .small()
                            .icon(IconName::Plus)
                            .label(t!("smart_routing.add_redirect").to_string())
                            .disabled(count >= MAX_MODEL_REDIRECTS)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.add_model_redirect(window, cx);
                            })),
                    ),
            )
            .child(
                v_flex()
                    .w_full()
                    .rounded(px(10.))
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.background)
                    .children(
                        self.smart_routing
                            .draft
                            .model_redirects
                            .iter()
                            .enumerate()
                            .map(|(i, r)| {
                                let enabled = r.enabled;
                                let builtin = is_builtin_model_redirect(
                                    &self.smart_routing.draft.model_redirects,
                                    i,
                                );
                                let issue = issues.get(i).and_then(|x| *x);
                                // AstrLink: blank fields only after a committed save attempt.
                                let shown_issue = issue.filter(|iss| {
                                    show_issues
                                        || !matches!(
                                            iss,
                                            ModelRedirectIssue::EmptyFrom
                                                | ModelRedirectIssue::EmptyTo
                                        )
                                });
                                let from_input = self.smart_routing.redirect_from.get(i).cloned();
                                let to_input = self.smart_routing.redirect_to.get(i).cloned();
                                let model_options = self.smart_routing.model_options.clone();
                                let view = cx.entity().downgrade();
                                v_flex()
                                    .id(SharedString::from(format!("sr-redir-{i}")))
                                    .w_full()
                                    .border_b_1()
                                    .border_color(theme.border.opacity(0.6))
                                    .child(
                                        h_flex()
                                            .w_full()
                                            .px(px(14.))
                                            .py(px(10.))
                                            .gap(px(12.))
                                            .items_start()
                                            .child(
                                                v_flex()
                                                    .gap(px(6.))
                                                    .items_start()
                                                    .child(
                                                        div()
                                                            .text_size(px(11.))
                                                            .text_color(theme.muted_foreground)
                                                            .child(
                                                                t!("smart_routing.col_enabled")
                                                                    .to_string(),
                                                            ),
                                                    )
                                                    .child(
                                                        Switch::new(SharedString::from(format!(
                                                            "sr-redir-on-{i}"
                                                        )))
                                                        .checked(enabled)
                                                        .xsmall()
                                                        .on_click(move |checked, window, cx| {
                                                            if let Some(entity) = view.upgrade() {
                                                                let checked = *checked;
                                                                entity.update(cx, |this, cx| {
                                                                    if let Some(row) = this
                                                                        .smart_routing
                                                                        .draft
                                                                        .model_redirects
                                                                        .get_mut(i)
                                                                    {
                                                                        row.enabled = checked;
                                                                    }
                                                                    this.smart_routing
                                                                        .editing_redirect = false;
                                                                    this.mark_routing_dirty(
                                                                        window, cx,
                                                                    );
                                                                });
                                                            }
                                                        }),
                                                    ),
                                            )
                                            .child(
                                                v_flex()
                                                    .flex_1()
                                                    .min_w(px(0.))
                                                    .gap(px(4.))
                                                    .child(
                                                        div()
                                                            .text_size(px(11.))
                                                            .text_color(theme.muted_foreground)
                                                            .child(if builtin {
                                                                t!(
                                                                    "smart_routing.builtin_codex_auto_review"
                                                                )
                                                                .to_string()
                                                            } else {
                                                                t!("smart_routing.col_from")
                                                                    .to_string()
                                                            }),
                                                    )
                                                    .when(builtin, |el| {
                                                        // AstrLink: built-in source is a fixed label, not an input.
                                                        el.child(
                                                            h_flex()
                                                                .w_full()
                                                                .h(px(32.))
                                                                .px(px(10.))
                                                                .rounded(px(6.))
                                                                .border_1()
                                                                .border_color(theme.border)
                                                                .bg(theme.muted.opacity(0.35))
                                                                .items_center()
                                                                .text_size(px(13.))
                                                                .text_color(theme.muted_foreground)
                                                                .child(r.from.clone()),
                                                        )
                                                    })
                                                    .when(!builtin, |el| {
                                                        el.when_some(from_input, |el, input| {
                                                            el.child(
                                                                self.render_redirect_model_combobox(
                                                                    i,
                                                                    true,
                                                                    SharedString::from(format!(
                                                                        "sr-redir-from-{i}"
                                                                    )),
                                                                    input,
                                                                    &model_options,
                                                                    false,
                                                                    cx,
                                                                ),
                                                            )
                                                        })
                                                    }),
                                            )
                                            .child(
                                                h_flex()
                                                    .pt(px(22.))
                                                    .h(px(32.))
                                                    .items_center()
                                                    .justify_center()
                                                    .flex_shrink_0()
                                                    .child(
                                                        Icon::new(IconName::ArrowRight)
                                                            .size(px(16.))
                                                            .text_color(theme.muted_foreground),
                                                    ),
                                            )
                                            .child(
                                                v_flex()
                                                    .flex_1()
                                                    .min_w(px(0.))
                                                    .gap(px(4.))
                                                    .child(
                                                        div()
                                                            .text_size(px(11.))
                                                            .text_color(theme.muted_foreground)
                                                            .child(
                                                                t!("smart_routing.col_to")
                                                                    .to_string(),
                                                            ),
                                                    )
                                                    .when_some(to_input, |el, input| {
                                                        el.child(
                                                            self.render_redirect_model_combobox(
                                                                i,
                                                                false,
                                                                SharedString::from(format!(
                                                                    "sr-redir-to-{i}"
                                                                )),
                                                                input,
                                                                &model_options,
                                                                false,
                                                                cx,
                                                            ),
                                                        )
                                                    }),
                                            )
                                            .child(
                                                h_flex()
                                                    .pt(px(22.))
                                                    .h(px(32.))
                                                    .items_center()
                                                    .when(!builtin, |el| {
                                                        el.child(
                                                            Button::new(SharedString::from(
                                                                format!("sr-redir-del-{i}"),
                                                            ))
                                                            .ghost()
                                                            .xsmall()
                                                            .icon(IconName::Close)
                                                            .on_click(cx.listener(
                                                                move |this, _, window, cx| {
                                                                    this.remove_model_redirect(
                                                                        i, window, cx,
                                                                    );
                                                                },
                                                            )),
                                                        )
                                                    }),
                                            ),
                                    )
                                    .when_some(shown_issue, |row, iss| {
                                        row.child(
                                            div()
                                                .w_full()
                                                .px(px(14.))
                                                .pb(px(10.))
                                                .child(
                                                    div()
                                                        .w_full()
                                                        .rounded(px(6.))
                                                        .border_1()
                                                        .border_color(theme.danger.opacity(0.25))
                                                        .bg(theme.danger.opacity(0.08))
                                                        .px(px(12.))
                                                        .py(px(8.))
                                                        .text_size(px(12.))
                                                        .text_color(theme.danger)
                                                        .child(redirect_issue_message(iss)),
                                                ),
                                        )
                                    })
                            }),
                    )
                    .when(count == 0, |el| {
                        el.child(
                            div()
                                .px(px(14.))
                                .py(px(24.))
                                .text_size(px(12.))
                                .text_color(theme.muted_foreground)
                                .child(t!("smart_routing.redirects_empty").to_string()),
                        )
                    }),
            )
    }

    /// AstrLink-style combobox: list under the input (same width), fuzzy filter, freeform Enter.
    fn render_redirect_model_combobox(
        &self,
        index: usize,
        is_from: bool,
        id: SharedString,
        input: Entity<InputState>,
        model_options: &[String],
        disabled: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme().clone();
        let open = !disabled && self.smart_routing.redirect_dropdown == Some((index, is_from));
        let current_value = input.read(cx).value().to_string();
        // AstrLink: opening shows every option; only typing after open filters.
        let filter = if open && self.smart_routing.redirect_filter_active {
            self.smart_routing.redirect_filter_query.clone()
        } else {
            String::new()
        };
        let filter_l = filter.to_lowercase();
        let filtered: Vec<String> = model_options
            .iter()
            .filter(|m| filter_l.is_empty() || m.to_lowercase().contains(&filter_l))
            .take(REDIRECT_MENU_MAX)
            .cloned()
            .collect();
        let no_match = filtered.is_empty() && self.smart_routing.redirect_filter_active;
        let view = cx.entity().downgrade();
        let input_for_toggle = input.clone();

        div()
            .id(SharedString::from(format!(
                "sr-redir-combo-{index}-{is_from}"
            )))
            .w_full()
            .relative()
            // Open on press in the field area (Focus alone can miss when already focused).
            .on_mouse_down(MouseButton::Left, {
                let view = view.clone();
                let input_for_open = input.clone();
                move |_, _, cx| {
                    if disabled {
                        return;
                    }
                    if let Some(entity) = view.upgrade() {
                        entity.update(cx, |this, cx| {
                            let key = (index, is_from);
                            // AstrLink: only showOptions when not already expanded —
                            // don't wipe an in-progress filter on caret clicks.
                            if this.smart_routing.redirect_dropdown == Some(key) {
                                return;
                            }
                            let baseline = input_for_open.read(cx).value().to_string();
                            this.open_redirect_model_dropdown(index, is_from, &baseline, cx);
                        });
                    }
                }
            })
            .child(
                Input::new(&input)
                    .cleanable(!disabled)
                    .disabled(disabled)
                    .suffix(
                        Button::new(id)
                            .ghost()
                            .xsmall()
                            .icon(IconName::ChevronDown)
                            .disabled(disabled)
                            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                // Avoid parent open + chevron toggle racing on the same click.
                                cx.stop_propagation();
                            })
                            .on_click({
                                let view = view.clone();
                                move |_, window, cx| {
                                    if let Some(entity) = view.upgrade() {
                                        entity.update(cx, |this, cx| {
                                            let key = (index, is_from);
                                            if this.smart_routing.redirect_dropdown == Some(key) {
                                                this.close_redirect_model_dropdown();
                                                cx.notify();
                                            } else {
                                                let baseline =
                                                    input_for_toggle.read(cx).value().to_string();
                                                this.open_redirect_model_dropdown(
                                                    index, is_from, &baseline, cx,
                                                );
                                                input_for_toggle.update(cx, |s, cx| {
                                                    s.focus(window, cx);
                                                });
                                            }
                                        });
                                    }
                                }
                            }),
                    ),
            )
            .when(open, |el| {
                let view = view.clone();
                el.child(
                    v_flex()
                        .id(SharedString::from(format!(
                            "sr-redir-menu-{index}-{is_from}"
                        )))
                        .absolute()
                        .top_full()
                        .left_0()
                        .right_0()
                        .mt(px(4.))
                        .w_full()
                        .max_h(px(256.))
                        .overflow_y_scroll()
                        .popover_style(cx)
                        .p(px(4.))
                        .gap(px(2.))
                        .occlude()
                        .on_mouse_down(MouseButton::Left, |_, _, cx| {
                            // Keep focus on the input while choosing a row.
                            cx.stop_propagation();
                        })
                        .children(filtered.into_iter().enumerate().map(|(row, model)| {
                            let chosen = model.clone();
                            let inp = input.clone();
                            let view = view.clone();
                            let selected = model == current_value;
                            div()
                                .id(SharedString::from(format!(
                                    "sr-redir-opt-{index}-{is_from}-{row}"
                                )))
                                .w_full()
                                .px(px(8.))
                                .py(px(6.))
                                .rounded(px(6.))
                                .cursor_pointer()
                                .text_size(px(13.))
                                .when(selected, |row| row.bg(theme.accent.opacity(0.35)))
                                .hover(|s| s.bg(theme.accent.opacity(0.25)))
                                .child(model)
                                .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                                    cx.stop_propagation();
                                    // AstrLink choose: close first so set_value/Focus cannot reopen.
                                    if let Some(entity) = view.upgrade() {
                                        entity.update(cx, |this, cx| {
                                            this.close_redirect_model_dropdown();
                                            this.smart_routing.editing_redirect = false;
                                            cx.notify();
                                        });
                                    }
                                    inp.update(cx, |s, cx| {
                                        s.set_value(chosen.clone(), window, cx);
                                    });
                                    if let Some(entity) = view.upgrade() {
                                        entity.update(cx, |this, cx| {
                                            this.mark_routing_dirty(window, cx);
                                        });
                                    }
                                })
                        }))
                        .when(no_match, |menu| {
                            menu.child(
                                div()
                                    .px(px(8.))
                                    .py(px(6.))
                                    .text_size(px(12.))
                                    .text_color(theme.muted_foreground)
                                    .child(t!("smart_routing.no_matching_models").to_string()),
                            )
                        }),
                )
            })
    }

    fn render_routing_recovery_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        // AstrLink RoutingSettingsPanel recovery tab:
        // Panel「失败恢复与切换」→ Switch + Select + max attempts
        // → Panel「重试次数与等待时间」→ Panel「推理内容修复」
        let theme = cx.theme().clone();
        let allow = self.smart_routing.draft.allow_unmatched_failover;
        let failover_only = self.smart_routing.draft.strategy == FailoverStrategy::FailoverOnly;
        let thinking = self
            .smart_routing
            .draft
            .default_failure_policy
            .thinking_signature_recovery
            .unwrap_or(true);
        let reasoning = self
            .smart_routing
            .draft
            .default_failure_policy
            .openai_reasoning_recovery
            .unwrap_or(true);
        let function_out = self
            .smart_routing
            .draft
            .default_failure_policy
            .openai_function_output_recovery
            .unwrap_or(false);
        let retry_hint = if failover_only {
            t!("smart_routing.retry_once_hint").to_string()
        } else {
            t!("smart_routing.retry_all_services_hint").to_string()
        };

        // Full-bleed panels (no max_w) so every card shares the same right edge.
        v_flex()
            .w_full()
            .gap(px(12.))
            // Panel 1: 失败恢复与切换
            .child(
                self.routing_settings_panel(
                    t!("smart_routing.order_title").to_string(),
                    t!("smart_routing.order_hint").to_string(),
                    v_flex()
                        .w_full()
                        .gap(px(16.))
                        .child(
                            v_flex()
                                .w_full()
                                .gap(px(8.))
                                .child(
                                    h_flex()
                                        .w_full()
                                        .gap(px(8.))
                                        .items_center()
                                        .child(
                                            Switch::new("sr-allow-failover")
                                                .checked(allow)
                                                .on_click(cx.listener(
                                                    |this, checked: &bool, window, cx| {
                                                        this.smart_routing
                                                            .draft
                                                            .allow_unmatched_failover = *checked;
                                                        this.mark_routing_dirty(window, cx);
                                                    },
                                                )),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(13.))
                                                .text_color(theme.foreground)
                                                .child(
                                                    t!("smart_routing.global_switch").to_string(),
                                                ),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .text_color(theme.muted_foreground)
                                        .child(t!("smart_routing.global_off_hint").to_string()),
                                ),
                        )
                        .child(
                            h_flex()
                                .w_full()
                                .gap(px(12.))
                                .items_start()
                                .child(
                                    v_flex()
                                        .flex_1()
                                        .w_full()
                                        .gap(px(6.))
                                        .min_w(px(0.))
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(theme.foreground)
                                                .child(t!("smart_routing.order").to_string()),
                                        )
                                        .child(
                                            Select::new(&self.smart_routing.strategy_select)
                                                .small()
                                                .w_full(),
                                        ),
                                )
                                .child(
                                    v_flex()
                                        .flex_1()
                                        .w_full()
                                        .gap(px(6.))
                                        .min_w(px(0.))
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(theme.foreground)
                                                .child(
                                                    t!("smart_routing.max_attempts").to_string(),
                                                ),
                                        )
                                        .child(
                                            Input::new(&self.smart_routing.max_attempts_input)
                                                .small()
                                                .cleanable(true)
                                                .w_full(),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(11.))
                                                .text_color(theme.muted_foreground)
                                                .child(
                                                    t!("smart_routing.max_attempts_hint")
                                                        .to_string(),
                                                ),
                                        ),
                                ),
                        )
                        .into_any_element(),
                    cx,
                ),
            )
            // Panel 2: 重试次数与等待时间 — same 2-col grid as AstrLink
            .child(
                self.routing_settings_panel(
                    t!("smart_routing.retry_title").to_string(),
                    retry_hint,
                    v_flex()
                        .w_full()
                        .gap(px(12.))
                        .child(
                            h_flex()
                                .w_full()
                                .gap(px(12.))
                                .items_start()
                                .child(self.routing_field_input(
                                    t!("smart_routing.max_retries").to_string(),
                                    Some(t!("smart_routing.max_retries_hint").to_string()),
                                    &self.smart_routing.max_retries_input,
                                    cx,
                                ))
                                .child(self.routing_field_input(
                                    t!("smart_routing.initial_delay").to_string(),
                                    None,
                                    &self.smart_routing.initial_delay_input,
                                    cx,
                                )),
                        )
                        .child(
                            // AstrLink: max_delay sits in left half of the 2-col grid.
                            h_flex()
                                .w_full()
                                .gap(px(12.))
                                .items_start()
                                .child(self.routing_field_input(
                                    t!("smart_routing.max_delay").to_string(),
                                    Some(t!("smart_routing.max_delay_hint").to_string()),
                                    &self.smart_routing.max_delay_input,
                                    cx,
                                ))
                                .child(div().flex_1().min_w(px(0.))),
                        )
                        .into_any_element(),
                    cx,
                ),
            )
            // Panel 3: 推理内容修复
            .child(
                self.routing_settings_panel(
                    t!("smart_routing.repair_title").to_string(),
                    t!("smart_routing.repair_hint").to_string(),
                    v_flex()
                        .w_full()
                        .gap(px(14.))
                        .child(self.routing_switch_field(
                            "sr-think-repair",
                            thinking,
                            t!("smart_routing.repair_thinking").to_string(),
                            t!("smart_routing.repair_thinking_hint").to_string(),
                            |draft, v| {
                                draft.default_failure_policy.thinking_signature_recovery = Some(v);
                            },
                            cx,
                        ))
                        .child(self.routing_switch_field(
                            "sr-reason-repair",
                            reasoning,
                            t!("smart_routing.repair_reasoning").to_string(),
                            t!("smart_routing.repair_reasoning_hint").to_string(),
                            |draft, v| {
                                draft.default_failure_policy.openai_reasoning_recovery = Some(v);
                            },
                            cx,
                        ))
                        .child(self.routing_switch_field(
                            "sr-function-repair",
                            function_out,
                            t!("smart_routing.repair_function").to_string(),
                            t!("smart_routing.repair_function_hint").to_string(),
                            |draft, v| {
                                draft.default_failure_policy.openai_function_output_recovery =
                                    Some(v);
                            },
                            cx,
                        ))
                        .into_any_element(),
                    cx,
                ),
            )
    }

    fn routing_settings_panel(
        &self,
        title: String,
        hint: String,
        body: impl IntoElement,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme().clone();
        v_flex()
            .w_full()
            .rounded(px(8.))
            .border_1()
            .border_color(theme.border)
            .bg(theme.background)
            .overflow_hidden()
            .child(
                v_flex()
                    .w_full()
                    .gap(px(4.))
                    .px(px(16.))
                    .py(px(12.))
                    .border_b_1()
                    .border_color(theme.border)
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
                            .child(hint),
                    ),
            )
            .child(div().w_full().p(px(16.)).child(body))
    }

    fn routing_field_input(
        &self,
        label: String,
        hint: Option<String>,
        input: &Entity<InputState>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme().clone();
        v_flex()
            .flex_1()
            .w_full()
            .gap(px(6.))
            .min_w(px(0.))
            .child(
                div()
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme.foreground)
                    .child(label),
            )
            .child(Input::new(input).small().cleanable(true).w_full())
            .when_some(hint, |this, hint| {
                this.child(
                    div()
                        .text_size(px(11.))
                        .text_color(theme.muted_foreground)
                        .child(hint),
                )
            })
    }

    fn routing_switch_field(
        &self,
        id: &'static str,
        checked: bool,
        label: String,
        hint: String,
        set: impl Fn(&mut RoutingSettings, bool) + 'static,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme().clone();
        // Label+hint stretch left; Switch sits in a fixed right column so every
        // row shares one vertical edge flush with the panel.
        h_flex()
            .w_full()
            .items_start()
            .justify_between()
            .gap(px(16.))
            .child(
                v_flex()
                    .flex_1()
                    .min_w(px(0.))
                    .gap(px(4.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.foreground)
                            .child(label),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(theme.muted_foreground)
                            .child(hint),
                    ),
            )
            .child(div().flex_shrink_0().pt(px(2.)).child(
                Switch::new(id).checked(checked).on_click(cx.listener(
                    move |this, checked: &bool, window, cx| {
                        set(&mut this.smart_routing.draft, *checked);
                        this.mark_routing_dirty(window, cx);
                    },
                )),
            ))
    }

    fn render_routing_rules_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        // AstrLink FailureRulesEditor: title + help + reset → table → add row.
        let theme = cx.theme().clone();
        let policy = &self.smart_routing.draft.default_failure_policy;
        let help_body = format!(
            "{}\n\n{}\n\n{}",
            t!("smart_routing.rules_hint"),
            t!("smart_routing.retry_rules_hint"),
            t!("smart_routing.other_errors")
        );

        let mut rows: Vec<(String, Option<String>, FailureAction)> = Vec::new();
        rows.push(("network_error".into(), None, policy.network_error));
        rows.push(("response_timeout".into(), None, policy.response_timeout));
        let mut codes: Vec<_> = policy.http_status.keys().cloned().collect();
        codes.sort_by(|a, b| {
            a.parse::<u16>()
                .unwrap_or(0)
                .cmp(&b.parse::<u16>().unwrap_or(0))
        });
        for code in codes {
            let action = policy
                .http_status
                .get(&code)
                .copied()
                .unwrap_or(FailureAction::Stop);
            rows.push((code.clone(), Some(code), action));
        }

        let raw_status = self
            .smart_routing
            .http_status_input
            .read(cx)
            .value()
            .to_string();
        let can_add = parse_http_status_code(&raw_status)
            .map(|code| !policy.http_status.contains_key(&code))
            .unwrap_or(false);

        v_flex()
            .w_full()
            .gap(px(8.))
            // Header: title + help | reset
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .child(
                        h_flex()
                            .items_center()
                            .gap(px(4.))
                            .min_w(px(0.))
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(theme.foreground)
                                    .child(t!("smart_routing.rules_title").to_string()),
                            )
                            .child(
                                Button::new("sr-rules-help")
                                    .ghost()
                                    .xsmall()
                                    .icon(CustomIcon::HelpCircle)
                                    .tooltip(help_body),
                            ),
                    )
                    .child(
                        Button::new("sr-rules-reset")
                            .ghost()
                            .small()
                            .icon(CustomIcon::RotateCw)
                            .label(t!("smart_routing.reset_section").to_string())
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.reset_routing_rules_section(window, cx);
                            })),
                    ),
            )
            // Table header
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .h(px(36.))
                    .border_b_1()
                    .border_color(theme.border)
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.muted_foreground)
                            .child(t!("smart_routing.error_type").to_string()),
                    )
                    .child(
                        div()
                            .w(px(96.))
                            .flex_shrink_0()
                            .text_center()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.muted_foreground)
                            .child(t!("smart_routing.allow_retry").to_string()),
                    )
                    .child(div().w(px(40.)).flex_shrink_0()),
            )
            // Rows
            .children(rows.into_iter().map(|(key, code, action)| {
                self.routing_rule_row(key, code, action, cx)
                    .into_any_element()
            }))
            // Add rule footer
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .gap(px(8.))
                    .pt(px(8.))
                    .border_t_1()
                    .border_color(theme.border)
                    .child(
                        div().w(px(128.)).child(
                            Input::new(&self.smart_routing.http_status_input)
                                .small()
                                .cleanable(true)
                                .w_full(),
                        ),
                    )
                    .child(
                        Button::new("sr-rules-add")
                            .outline()
                            .small()
                            .icon(IconName::Plus)
                            .label(t!("smart_routing.add_rule").to_string())
                            .disabled(!can_add)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.try_add_routing_http_status_rule(window, cx);
                            })),
                    ),
            )
    }

    fn routing_rule_row(
        &self,
        key: String,
        code: Option<String>,
        action: FailureAction,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme().clone();
        let allow = action != FailureAction::Stop;
        let is_http = code.is_some();
        let label_el: gpui::AnyElement = if let Some(ref code) = code {
            let status = http_status_label(code);
            h_flex()
                .items_baseline()
                .gap(px(12.))
                .flex_wrap()
                .child(
                    div()
                        .font_family("Menlo")
                        .text_size(px(12.))
                        .text_color(theme.foreground)
                        .child(t!("smart_routing.http_code", code = code.as_str()).to_string()),
                )
                .when(!status.is_empty(), |row| {
                    row.child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme.muted_foreground)
                            .child(status),
                    )
                })
                .into_any_element()
        } else {
            let label = if key == "network_error" {
                t!("smart_routing.network_error").to_string()
            } else {
                t!("smart_routing.response_timeout").to_string()
            };
            div()
                .text_size(px(12.))
                .text_color(theme.foreground)
                .child(label)
                .into_any_element()
        };
        let code_for_switch = code.clone();
        let key_for_switch = key.clone();
        let code_for_remove = code.clone();

        h_flex()
            .id(SharedString::from(format!("sr-rule-{key}")))
            .w_full()
            .items_center()
            .h(px(36.))
            .border_b_1()
            .border_color(theme.border.opacity(0.7))
            .child(div().flex_1().min_w(px(0.)).child(label_el))
            .child(
                h_flex()
                    .w(px(96.))
                    .flex_shrink_0()
                    .items_center()
                    .justify_center()
                    .child(
                        Switch::new(SharedString::from(format!("sr-rule-sw-{key}")))
                            .checked(allow)
                            .xsmall()
                            .on_click(cx.listener(move |this, checked: &bool, window, cx| {
                                let next = if *checked {
                                    FailureAction::RetryAndFailover
                                } else {
                                    FailureAction::Stop
                                };
                                let policy = &mut this.smart_routing.draft.default_failure_policy;
                                if let Some(ref code) = code_for_switch {
                                    policy.http_status.insert(code.clone(), next);
                                } else if key_for_switch == "network_error" {
                                    policy.network_error = next;
                                } else {
                                    policy.response_timeout = next;
                                }
                                this.mark_routing_dirty(window, cx);
                            })),
                    ),
            )
            .child(
                h_flex()
                    .w(px(40.))
                    .flex_shrink_0()
                    .items_center()
                    .justify_center()
                    .when(is_http, |el| {
                        el.child(
                            Button::new(SharedString::from(format!(
                                "sr-rule-del-{}",
                                code_for_remove.as_deref().unwrap_or("")
                            )))
                            .ghost()
                            .xsmall()
                            .icon(IconName::Close)
                            .tooltip(
                                t!(
                                    "smart_routing.remove_code",
                                    code = code_for_remove.as_deref().unwrap_or("")
                                )
                                .to_string(),
                            )
                            .on_click(cx.listener(
                                move |this, _, window, cx| {
                                    if let Some(ref code) = code_for_remove {
                                        this.smart_routing
                                            .draft
                                            .default_failure_policy
                                            .http_status
                                            .remove(code);
                                        this.mark_routing_dirty(window, cx);
                                    }
                                },
                            )),
                        )
                    }),
            )
    }

    fn render_routing_session_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let enabled = self.smart_routing.draft.channel_stickiness.enabled;
        v_flex()
            .w_full()
            .gap(px(16.))
            .child(self.routing_section_title(
                t!("smart_routing.session_title").to_string(),
                t!("smart_routing.session_hint").to_string(),
                cx,
            ))
            .child(
                h_flex()
                    .gap(px(10.))
                    .items_center()
                    .child(
                        Checkbox::new("sr-stickiness")
                            .checked(enabled)
                            .on_click(cx.listener(|this, checked: &bool, window, cx| {
                                this.smart_routing.draft.channel_stickiness.enabled = *checked;
                                this.mark_routing_dirty(window, cx);
                            })),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .child(t!("smart_routing.stickiness_enabled").to_string()),
                    ),
            )
            .child(self.routing_labeled_input(
                t!("smart_routing.ttl_minutes").to_string(),
                &self.smart_routing.ttl_minutes_input,
                cx,
            ))
    }

    fn render_routing_identity_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap(px(16.))
            .child(self.routing_section_title(
                t!("smart_routing.identity_title").to_string(),
                t!("smart_routing.identity_hint").to_string(),
                cx,
            ))
            .child(self.routing_checkbox_row(
                "sr-id-codex",
                self.smart_routing.draft.codex_identity_enforcement,
                t!("smart_routing.identity_codex").to_string(),
                |draft, v| draft.codex_identity_enforcement = v,
                cx,
            ))
            .child(self.routing_checkbox_row(
                "sr-id-claude",
                self.smart_routing.draft.claude_identity_enforcement,
                t!("smart_routing.identity_claude").to_string(),
                |draft, v| draft.claude_identity_enforcement = v,
                cx,
            ))
            .child(self.routing_checkbox_row(
                "sr-id-grok",
                self.smart_routing.draft.grok_identity_enforcement,
                t!("smart_routing.identity_grok").to_string(),
                |draft, v| draft.grok_identity_enforcement = v,
                cx,
            ))
    }

    fn routing_section_title(
        &self,
        title: String,
        hint: String,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme().clone();
        v_flex()
            .gap(px(4.))
            .child(
                div()
                    .text_size(px(14.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(title),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(theme.muted_foreground)
                    .child(hint),
            )
    }

    fn routing_labeled_input(
        &self,
        label: String,
        input: &Entity<InputState>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme().clone();
        v_flex()
            .gap(px(6.))
            .w_full()
            .child(
                div()
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme.foreground)
                    .child(label),
            )
            .child(Input::new(input).small().cleanable(true))
    }

    fn routing_checkbox_row(
        &self,
        id: &'static str,
        checked: bool,
        label: String,
        set: impl Fn(&mut RoutingSettings, bool) + 'static,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        h_flex()
            .gap(px(10.))
            .items_center()
            .child(Checkbox::new(id).checked(checked).on_click(cx.listener(
                move |this, checked: &bool, window, cx| {
                    set(&mut this.smart_routing.draft, *checked);
                    this.mark_routing_dirty(window, cx);
                },
            )))
            .child(div().text_size(px(13.)).child(label))
    }
}
