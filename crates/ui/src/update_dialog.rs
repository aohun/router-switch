use std::rc::Rc;

use domain::AppRelease;
use gpui::{
    div, prelude::FluentBuilder as _, px, rgb, App, FontWeight, IntoElement, ParentElement as _,
    Styled as _, Window,
};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    h_flex,
    scroll::ScrollableElement as _,
    v_flex, ActiveTheme as _, Sizable as _, WindowExt as _,
};
use rust_i18n::t;

/// Helper function to open URLs in the system default browser across macOS, Windows, and Linux.
pub fn open_url(url: &str) {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(url).spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn();
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
}

/// Opens the Sparkle-styled software upgrade prompt dialog.
pub fn open_app_update_dialog<FAuto, FSkip, FInstall>(
    window: &mut Window,
    cx: &mut App,
    release: AppRelease,
    current_version: String,
    auto_check_update: bool,
    on_toggle_auto_check: FAuto,
    on_skip_version: FSkip,
    on_install_update: FInstall,
) where
    FAuto: Fn(bool, &mut Window, &mut App) + 'static,
    FSkip: Fn(String, &mut Window, &mut App) + 'static,
    FInstall: Fn(String, &mut Window, &mut App) + 'static,
{
    let app_name = "AICWITCH";
    let theme = cx.theme().clone();

    let release_clone = release.clone();
    let curr_ver = current_version.clone();
    let on_toggle_auto_check = Rc::new(on_toggle_auto_check);
    let on_skip_version = Rc::new(on_skip_version);
    let on_install_update = Rc::new(on_install_update);

    window.open_dialog(cx, move |dialog, _window, _cx| {
        let release = release_clone.clone();
        let download_url = release.download_url.clone();
        let release_version = release.version.clone();

        let cb_toggle = on_toggle_auto_check.clone();
        let cb_skip = on_skip_version.clone();
        let cb_install = on_install_update.clone();

        dialog
            .width(px(560.))
            .overlay_closable(false)
            .close_button(false)
            // 1. Header with App Logo, Title and Subtitle
            .title(
                h_flex()
                    .w_full()
                    .items_start()
                    .gap(px(14.))
                    .p_1()
                    .child(
                        // Green gradient rounded App Icon container
                        div()
                            .size(px(52.))
                            .rounded(px(12.))
                            .bg(rgb(0x34D399))
                            .flex()
                            .items_center()
                            .justify_center()
                            .shadow_md()
                            .flex_shrink_0()
                            .child(
                                div()
                                    .text_size(px(28.))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(0xFFFFFF))
                                    .child("A"),
                            ),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .gap(px(3.))
                            .child(
                                div()
                                    .text_size(px(16.))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(theme.foreground)
                                    .child(format!("A new version of {app_name} is available!")),
                            )
                            .child(
                                div()
                                    .text_size(px(12.5))
                                    .line_height(px(18.))
                                    .text_color(theme.muted_foreground)
                                    .child(format!(
                                        "{app_name} {} is now available—you have {}. Would you like to download it now?",
                                        release.version, curr_ver
                                    )),
                            ),
                    ),
            )
            // 2. Release Notes Box (Scrollable card with border)
            .child(
                v_flex()
                    .w_full()
                    .max_h(px(230.))
                    .p(px(14.))
                    .gap(px(12.))
                    .rounded(px(8.))
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.secondary.opacity(0.2))
                    .when(!release.release_notes_zh.is_empty(), |this| {
                        this.child(
                            v_flex()
                                .w_full()
                                .gap(px(6.))
                                .child(
                                    div()
                                        .text_size(px(14.5))
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(theme.foreground)
                                        .child("新变化"),
                                )
                                .children(release.release_notes_zh.iter().map(|item| {
                                    h_flex()
                                        .w_full()
                                        .items_start()
                                        .gap(px(8.))
                                        .child(
                                            div()
                                                .text_size(px(13.))
                                                .text_color(theme.muted_foreground)
                                                .child("•"),
                                        )
                                        .child(
                                            div()
                                                .flex_1()
                                                .text_size(px(13.))
                                                .line_height(px(19.))
                                                .text_color(theme.foreground)
                                                .child(item.clone()),
                                        )
                                })),
                        )
                    })
                    .when(!release.release_notes_en.is_empty(), |this| {
                        this.child(
                            v_flex()
                                .w_full()
                                .gap(px(6.))
                                .child(
                                    div()
                                        .text_size(px(14.5))
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(theme.foreground)
                                        .child("What's new"),
                                )
                                .children(release.release_notes_en.iter().map(|item| {
                                    h_flex()
                                        .w_full()
                                        .items_start()
                                        .gap(px(8.))
                                        .child(
                                            div()
                                                .text_size(px(13.))
                                                .text_color(theme.muted_foreground)
                                                .child("•"),
                                        )
                                        .child(
                                            div()
                                                .flex_1()
                                                .text_size(px(13.))
                                                .line_height(px(19.))
                                                .text_color(theme.foreground)
                                                .child(item.clone()),
                                        )
                                })),
                        )
                    })
                    .when(
                        release.release_notes_zh.is_empty() && release.release_notes_en.is_empty(),
                        |this| {
                            this.child(
                                div()
                                    .text_size(px(13.))
                                    .line_height(px(19.))
                                    .text_color(theme.foreground)
                                    .child(if release.body.is_empty() {
                                        "Bug fixes and performance improvements.".to_string()
                                    } else {
                                        release.body.clone()
                                    }),
                            )
                        },
                    )
                    .overflow_y_scrollbar(),
            )
            // 3. Footer: Checkbox on left + 3 Action Buttons on right
            .footer(move |_ok_btn, _cancel_btn, _window, _cx| {
                let download_url_btn = download_url.clone();
                let skip_ver = release_version.clone();

                let toggle_fn = cb_toggle.clone();
                let skip_fn = cb_skip.clone();
                let install_fn = cb_install.clone();

                vec![
                    h_flex()
                        .w_full()
                        .justify_between()
                        .items_center()
                        .gap(px(12.))
                        .child(
                            Checkbox::new("auto-update-checkbox")
                                .checked(auto_check_update)
                                .label(t!("update.auto_update_checkbox").to_string())
                                .on_click(move |checked, window, cx| {
                                    toggle_fn(*checked, window, cx);
                                }),
                        )
                        .child(
                            h_flex()
                                .items_center()
                                .gap(px(8.))
                                .child(
                                    Button::new("skip-this-version")
                                        .label(t!("update.skip_version"))
                                        .ghost()
                                        .small()
                                        .on_click(move |_, window, cx| {
                                            skip_fn(skip_ver.clone(), window, cx);
                                            window.close_dialog(cx);
                                        }),
                                )
                                .child(
                                    Button::new("remind-me-later")
                                        .label(t!("update.remind_later"))
                                        .outline()
                                        .small()
                                        .on_click(|_, window: &mut Window, cx| {
                                            window.close_dialog(cx);
                                        }),
                                )
                                .child(
                                    Button::new("install-update")
                                        .label(t!("update.install_update"))
                                        .primary()
                                        .small()
                                        .on_click(move |_, window, cx| {
                                            let url = download_url_btn.clone();
                                            open_url(&url);
                                            install_fn(url, window, cx);
                                            window.close_dialog(cx);
                                        }),
                                ),
                        )
                        .into_any_element(),
                ]
            })
    });
}
