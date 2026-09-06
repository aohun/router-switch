#[cfg(target_os = "macos")]
pub mod macos {
    use cocoa::appkit::{NSApp, NSImage, NSMenu, NSMenuItem};
    use cocoa::base::{id, nil, NO, YES};
    use cocoa::foundation::{NSAutoreleasePool, NSData, NSSize, NSString};
    use objc::declare::ClassDecl;
    use objc::runtime::{Class, Object, Sel};
    use objc::{class, msg_send, sel, sel_impl};
    use std::sync::Once;

    static INIT: Once = Once::new();
    static mut STATUS_ITEM: id = nil;
    static mut TARGET: id = nil;

    extern "C" fn show_main_window(_this: &Object, _sel: Sel, _sender: id) {
        unsafe {
            let app = NSApp();
            let () = msg_send![app, unhide: nil];
            let () = msg_send![app, activateIgnoringOtherApps: YES];
            let windows: id = msg_send![app, windows];
            if windows != nil {
                let count: usize = msg_send![windows, count];
                for i in 0..count {
                    let window: id = msg_send![windows, objectAtIndex: i];
                    let () = msg_send![window, makeKeyAndOrderFront: nil];
                }
            }
        }
    }

    extern "C" fn quit_app(_this: &Object, _sel: Sel, _sender: id) {
        unsafe {
            let app = NSApp();
            let () = msg_send![app, terminate: nil];
        }
    }

    fn register_target_class() -> &'static Class {
        static mut CLS: Option<&'static Class> = None;
        static ONCE: Once = Once::new();
        ONCE.call_once(|| {
            let superclass = class!(NSObject);
            let mut decl = ClassDecl::new("RouterSwitchTrayTarget", superclass)
                .expect("failed to declare RouterSwitchTrayTarget");
            unsafe {
                decl.add_method(
                    sel!(showMainWindow:),
                    show_main_window as extern "C" fn(&Object, Sel, id),
                );
                decl.add_method(sel!(quitApp:), quit_app as extern "C" fn(&Object, Sel, id));
            }
            unsafe {
                CLS = Some(decl.register());
            }
        });
        unsafe { CLS.unwrap() }
    }

    pub fn setup_tray() {
        INIT.call_once(|| unsafe {
            let pool = NSAutoreleasePool::new(nil);

            let target_class = register_target_class();
            let target: id = msg_send![target_class, new];
            TARGET = target;

            let status_bar: id = msg_send![class!(NSStatusBar), systemStatusBar];
            // -1.0 is NSVariableStatusItemLength
            let status_item: id = msg_send![status_bar, statusItemWithLength: -1.0f64];
            let () = msg_send![status_item, retain];
            STATUS_ITEM = status_item;

            let button: id = msg_send![status_item, button];
            if button != nil {
                let icon_bytes: &[u8] = include_bytes!("../../../assets/icon.png");
                let data = NSData::dataWithBytes_length_(
                    nil,
                    icon_bytes.as_ptr() as *const std::ffi::c_void,
                    icon_bytes.len() as u64,
                );
                let image = NSImage::initWithData_(NSImage::alloc(nil), data);
                if image != nil {
                    let () = msg_send![image, setSize: NSSize::new(18.0, 18.0)];
                    let () = msg_send![image, setTemplate: NO];
                    let () = msg_send![button, setImage: image];
                    let () = msg_send![button, setImagePosition: 2]; // NSImageOnly
                }
            }

            let menu = NSMenu::new(nil).autorelease();
            let () = msg_send![menu, setAutoenablesItems: NO];

            let title_open = NSString::alloc(nil).init_str("打开 Router Switch");
            let item_open = NSMenuItem::alloc(nil)
                .initWithTitle_action_keyEquivalent_(
                    title_open,
                    sel!(showMainWindow:),
                    NSString::alloc(nil).init_str(""),
                )
                .autorelease();
            let () = msg_send![item_open, setTarget: target];
            let () = msg_send![item_open, setEnabled: YES];
            let () = msg_send![menu, addItem: item_open];

            let separator: id = msg_send![class!(NSMenuItem), separatorItem];
            let () = msg_send![menu, addItem: separator];

            let title_quit = NSString::alloc(nil).init_str("退出 Router Switch");
            let item_quit = NSMenuItem::alloc(nil)
                .initWithTitle_action_keyEquivalent_(
                    title_quit,
                    sel!(quitApp:),
                    NSString::alloc(nil).init_str("q"),
                )
                .autorelease();
            let () = msg_send![item_quit, setTarget: target];
            let () = msg_send![item_quit, setEnabled: YES];
            let () = msg_send![menu, addItem: item_quit];

            let () = msg_send![status_item, setMenu: menu];

            let () = msg_send![pool, drain];
        });
    }
}

#[cfg(not(target_os = "macos"))]
pub mod non_macos {
    pub fn setup_tray() {}
}

pub fn setup_tray() {
    #[cfg(target_os = "macos")]
    macos::setup_tray();
    #[cfg(not(target_os = "macos"))]
    non_macos::setup_tray();
}
