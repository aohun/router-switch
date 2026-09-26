mod app_view;
pub mod assets;
mod insights_view;
mod theme;
pub mod tray;
pub mod update_dialog;
pub mod usage_service;
mod sessions_workbench;

use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::OnceLock;

use gpui::{
    px, size, App, AppContext, Application, Bounds, TitlebarOptions, WindowBackgroundAppearance,
    WindowBounds, WindowKind, WindowOptions,
};
use gpui_component::Root;

use crate::app_view::RouterApp;
use crate::assets::AppAssets;

rust_i18n::i18n!("locales", fallback = "zh-CN");

static DEEPLINK_CHANNEL: OnceLock<(Sender<String>, std::sync::Mutex<Receiver<String>>)> =
    OnceLock::new();

pub fn get_deeplink_channel() -> &'static (Sender<String>, std::sync::Mutex<Receiver<String>>) {
    DEEPLINK_CHANNEL.get_or_init(|| {
        let (tx, rx) = channel();
        (tx, std::sync::Mutex::new(rx))
    })
}

pub fn send_deeplink(url: String) {
    let (tx, _) = get_deeplink_channel();
    let _ = tx.send(url);
}

#[cfg(target_os = "macos")]
fn setup_dock_icon() {
    use cocoa::appkit::{NSApp, NSApplication, NSImage};
    use cocoa::base::nil;
    use cocoa::foundation::NSData;

    unsafe {
        let icon_bytes: &[u8] = include_bytes!("../../../assets/icon.icns");
        let data = NSData::dataWithBytes_length_(
            nil,
            icon_bytes.as_ptr() as *const std::ffi::c_void,
            icon_bytes.len() as u64,
        );
        let image = NSImage::initWithData_(NSImage::alloc(nil), data);
        if image != nil {
            let app = NSApp();
            app.setApplicationIconImage_(image);
        }
    }
}

pub fn run() {
    let (tx, _) = get_deeplink_channel();
    let tx_clone = tx.clone();

    // Catch CLI argument deep links on launch
    for arg in std::env::args().skip(1) {
        if arg.starts_with("router-switch://")
            || arg.starts_with("ccswitch://")
            || arg.starts_with("routerswitch://")
        {
            let _ = tx.send(arg);
        }
    }

    let app = Application::new().with_assets(AppAssets);
    app.on_open_urls(move |urls| {
        for url in urls {
            let _ = tx_clone.send(url);
        }
    });

    app.run(|cx: &mut App| {
        #[cfg(target_os = "macos")]
        setup_dock_icon();

        tray::setup_tray();

        gpui_component::init(cx);
        theme::apply_palette(cx);
        cx.activate(true);

        // Align with Wake defaults: MAIN_SIZE 1180×760, min 940×620.
        let window_size = size(px(1180.), px(760.));
        let bounds = Bounds::centered(None, window_size, cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: Some("Router Switch".into()),
                appears_transparent: true,
                traffic_light_position: Some(gpui::point(px(16.), px(16.))),
            }),
            // Wake uses default Opaque. Blurred forces NSVisualEffectView /
            // backdrop blur on every live-resize frame and is a major lag source.
            window_background: WindowBackgroundAppearance::Opaque,
            window_min_size: Some(size(px(940.), px(620.))),
            kind: WindowKind::Normal,
            ..Default::default()
        };

        cx.open_window(options, |window, cx| {
            let app = cx.new(|cx| RouterApp::new(window, cx));
            cx.new(|cx| Root::new(app, window, cx))
        })
        .expect("failed to open window");
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn usage_script_guide_keeps_template_variables_literal() {
        rust_i18n::set_locale("zh-CN");
        let tips = rust_i18n::t!("usage_script.guide_tips").to_string();
        assert!(tips.contains("{{apiKey}}"), "tips: {tips}");
        assert!(tips.contains("{{baseUrl}}"), "tips: {tips}");
        let sample = rust_i18n::t!("usage_script.guide_sample").to_string();
        assert!(sample.contains("{{baseUrl}}/api/usage"), "sample: {sample}");
        assert!(sample.contains('\n'), "sample should be multi-line");
    }

    #[test]
    fn test_i18n_locales() {
        println!("Available locales: {:?}", rust_i18n::available_locales!());
        rust_i18n::set_locale("zh-CN");
        println!("Current locale: {}", &*rust_i18n::locale());
        println!(
            "general.theme_light in zh-CN: {}",
            rust_i18n::t!("general.theme_light")
        );
        println!(
            "settings.general in zh-CN: {}",
            rust_i18n::t!("settings.general")
        );
        rust_i18n::set_locale("en");
        println!("Current locale: {}", &*rust_i18n::locale());
        println!(
            "general.theme_light in en: {}",
            rust_i18n::t!("general.theme_light")
        );
        println!(
            "settings.general in en: {}",
            rust_i18n::t!("settings.general")
        );
    }
}
