//! Native macOS material; the system keeps ownership of window controls.
#![allow(unsafe_code)]
use adars::settings::Backdrop;
use objc2_app_kit::{
    NSAppearance, NSAppearanceCustomization, NSAppearanceNameAqua, NSAppearanceNameDarkAqua, NSView,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

pub fn configure(
    window: &eframe::Frame,
    enabled: bool,
    _backdrop: Backdrop,
    dark: bool,
) -> Result<(), String> {
    let RawWindowHandle::AppKit(handle) =
        window.window_handle().map_err(|e| e.to_string())?.as_raw()
    else {
        return Err("No macOS window".into());
    };
    // SAFETY: eframe invokes updates on the AppKit main thread. The NSView is
    // borrowed only during this call, while Frame owns the live native window.
    unsafe {
        let view = &*handle.ns_view.as_ptr().cast::<NSView>();
        let name = if dark {
            NSAppearanceNameDarkAqua
        } else {
            NSAppearanceNameAqua
        };
        let appearance = NSAppearance::appearanceNamed(name);
        view.setAppearance(appearance.as_deref());
        if let Some(native) = view.window() {
            native.setAppearance(appearance.as_deref());
            native.setTitlebarAppearsTransparent(enabled);
        }
    }
    if enabled {
        window_vibrancy::apply_vibrancy(
            window,
            window_vibrancy::NSVisualEffectMaterial::UnderWindowBackground,
            None,
            None,
        )
        .map_err(|e| e.to_string())
    } else {
        window_vibrancy::clear_vibrancy(window)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}
