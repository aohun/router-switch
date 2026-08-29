mod app_view;
pub mod assets;
mod theme;
pub mod update_dialog;
pub mod usage_service;

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

        gpui_component::init(cx);
        theme::apply_palette(cx);
        cx.activate(true);

        let window_size = size(px(1213.), px(816.));
        let bounds = Bounds::centered(None, window_size, cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: Some("Router Switch".into()),
                appears_transparent: true,
                traffic_light_position: Some(gpui::point(px(16.), px(16.))),
            }),
            window_background: WindowBackgroundAppearance::Blurred,
            window_min_size: Some(size(px(880.), px(600.))),
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
