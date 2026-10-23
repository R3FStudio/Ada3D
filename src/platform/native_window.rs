//! Native caption, DWM backdrop, and monitor-work-area positioning.
#![allow(unsafe_code)]
use adars::settings::Backdrop;

#[cfg(target_os = "windows")]
mod windows {
    use super::Backdrop;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::{
        Foundation::{HWND, RECT},
        Graphics::{
            Dwm::*,
            Gdi::{
                GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromRect,
                MonitorFromWindow,
            },
        },
        UI::{
            Controls::MARGINS,
            WindowsAndMessaging::{
                GWL_STYLE, GetWindowLongPtrW, GetWindowRect, SWP_NOACTIVATE, SWP_NOSIZE,
                SWP_NOZORDER, SetWindowPos, WS_CAPTION,
            },
        },
    };

    fn hwnd(window: &impl HasWindowHandle) -> Result<HWND, String> {
        match window.window_handle().map_err(|e| e.to_string())?.as_raw() {
            RawWindowHandle::Win32(handle) => Ok(handle.hwnd.get() as HWND),
            _ => Err("This is not a Win32 window.".into()),
        }
    }
    fn attribute(handle: HWND, key: i32, value: i32) -> Result<(), String> {
        // SAFETY: handle is borrowed from the live eframe window; DWM copies this
        // correctly sized, initialized 32-bit value before returning.
        let result = unsafe {
            DwmSetWindowAttribute(
                handle,
                key as u32,
                (&value as *const i32).cast(),
                size_of::<i32>() as u32,
            )
        };
        if result < 0 {
            Err(format!("DWM attribute {key}: 0x{:08X}", result as u32))
        } else {
            Ok(())
        }
    }
    pub fn configure(
        window: &impl HasWindowHandle,
        enabled: bool,
        backdrop: Backdrop,
        dark: bool,
    ) -> Result<(), String> {
        let handle = hwnd(window)?;
        attribute(handle, DWMWA_USE_IMMERSIVE_DARK_MODE, i32::from(dark))?;
        // Preserve Mica. Acrylic suppresses the separate caption-color layer so
        // its backdrop also shows behind DWM-rendered native buttons.
        let [r, g, b] = caption_surface(dark);
        let caption_color = if enabled && backdrop == Backdrop::Acrylic {
            DWMWA_COLOR_NONE as i32
        } else {
            i32::from(r) | (i32::from(g) << 8) | (i32::from(b) << 16)
        };
        attribute(handle, DWMWA_CAPTION_COLOR, caption_color)?;
        let edge = if enabled { -1 } else { 0 };
        let margins = MARGINS {
            cxLeftWidth: edge,
            cxRightWidth: edge,
            cyTopHeight: if enabled {
                -1
            } else {
                // The custom client extends into the caption even with effects
                // off. Keep DWM rendering its real controls in that strip.
                let dpi = unsafe { windows_sys::Win32::UI::HiDpi::GetDpiForWindow(handle) }.max(96);
                (38 * dpi / 96) as i32
            },
            cyBottomHeight: edge,
        };
        // SAFETY: margins is initialized and lives through the synchronous call;
        // eframe owns the valid HWND for the duration of this function.
        let result = unsafe { DwmExtendFrameIntoClientArea(handle, &margins) };
        if result < 0 {
            return Err(format!("DWM frame extension: 0x{:08X}", result as u32));
        }
        if enabled {
            match backdrop {
                Backdrop::Acrylic => window_vibrancy::apply_acrylic(window, Some((0, 0, 0, 0))),
                Backdrop::Mica => window_vibrancy::apply_mica(window, Some(dark)),
            }
            .map_err(|e| e.to_string())?;
        } else {
            window_vibrancy::clear_acrylic(window).map_err(|e| e.to_string())?;
        }
        let _ = attribute(handle, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND);
        Ok(())
    }

    pub fn splash_frame(window: &impl HasWindowHandle) -> Result<(), String> {
        // Backdrop APIs can re-enable DWM captions on undecorated windows.
        // Keep the startup surface free of native controls until the editor
        // installs its decorated frame.
        attribute(hwnd(window)?, DWMWA_NCRENDERING_POLICY, DWMNCRP_DISABLED)
    }

    pub fn caption_controls_rect(window: &impl HasWindowHandle, scale: f32) -> Option<[f32; 4]> {
        use windows_sys::Win32::{Foundation::POINT, Graphics::Gdi::ClientToScreen};
        let handle = hwnd(window).ok()?;
        let mut buttons = RECT::default();
        let mut outer = RECT::default();
        let mut client = POINT::default();
        // SAFETY: the UI thread borrows a live window. All output structures
        // are initialized, correctly sized and retained only for these calls.
        unsafe {
            if DwmGetWindowAttribute(
                handle,
                DWMWA_CAPTION_BUTTON_BOUNDS as u32,
                (&mut buttons as *mut RECT).cast(),
                size_of::<RECT>() as u32,
            ) < 0
                || GetWindowRect(handle, &mut outer) == 0
                || ClientToScreen(handle, &mut client) == 0
                || buttons.right <= buttons.left
                || buttons.bottom <= buttons.top
            {
                return None;
            }
        }
        let dx = client.x - outer.left;
        let dy = client.y - outer.top;
        let scale = scale.max(0.5);
        Some([
            (buttons.left - dx) as f32 / scale,
            (buttons.top - dy) as f32 / scale,
            (buttons.right - dx) as f32 / scale,
            (buttons.bottom - dy) as f32 / scale,
        ])
    }

    pub fn header_top_inset(window: &impl HasWindowHandle, scale: f32) -> f32 {
        use windows_sys::Win32::{
            Foundation::POINT, Graphics::Gdi::ClientToScreen, UI::WindowsAndMessaging::IsZoomed,
        };
        let Ok((handle, _, work)) = geometry(window) else {
            return 0.0;
        };
        let mut client = POINT::default();
        // SAFETY: the borrowed HWND is live and client is a writable output.
        unsafe {
            if IsZoomed(handle) == 0 || ClientToScreen(handle, &mut client) == 0 {
                return 0.0;
            }
        }
        (work.top - client.y).max(0) as f32 / scale.max(0.5)
    }

    const HEADER_ID: usize = 0xADA3D;
    struct HeaderState {
        scale: f32,
        drag: Vec<[f32; 4]>,
    }
    unsafe extern "system" fn header_proc(
        handle: HWND,
        message: u32,
        wparam: usize,
        lparam: isize,
        id: usize,
        data: usize,
    ) -> isize {
        use windows_sys::Win32::UI::{
            Shell::{DefSubclassProc, RemoveWindowSubclass},
            WindowsAndMessaging::*,
        };
        // SAFETY: comctl32 calls this on the owning UI thread. 'data' is the Box
        // registered by install_header, and is released once on WM_NCDESTROY.
        unsafe {
            if message == WM_NCDESTROY {
                RemoveWindowSubclass(handle, Some(header_proc), id);
                drop(Box::from_raw(data as *mut HeaderState));
                return DefSubclassProc(handle, message, wparam, lparam);
            }
            if message == WM_NCCALCSIZE && wparam != 0 {
                let params = &mut *(lparam as *mut NCCALCSIZE_PARAMS);
                let proposed = params.rgrc[0];
                let original_top = params.rgrc[0].top;
                DefSubclassProc(handle, message, wparam, lparam);
                params.rgrc[0].top = original_top;
                if IsZoomed(handle) != 0 {
                    let params = &mut *(lparam as *mut NCCALCSIZE_PARAMS);
                    let mut monitor = MONITORINFO {
                        cbSize: size_of::<MONITORINFO>() as u32,
                        ..Default::default()
                    };
                    if GetMonitorInfoW(
                        MonitorFromRect(&proposed, MONITOR_DEFAULTTONEAREST),
                        &mut monitor,
                    ) != 0
                    {
                        params.rgrc[0] = monitor.rcWork;
                        // Keep the client flush with the outer top. A top
                        // non-client inset prevents DWM caption hover rendering
                        // on maximized custom frames. Layout accounts for the
                        // offscreen resize border separately.
                        params.rgrc[0].top = original_top;
                    }
                }
                return 0;
            }
            // Preserve host lifecycle and geometry events before DWM hit testing.
            if matches!(
                message,
                WM_SYSCOMMAND | WM_CLOSE | WM_SIZE | WM_DPICHANGED | WM_WINDOWPOSCHANGED
            ) {
                return DefSubclassProc(handle, message, wparam, lparam);
            }
            let mut result = 0;
            let handled = DwmDefWindowProc(handle, message, wparam, lparam, &mut result) != 0;
            if handled
                && (message != WM_NCHITTEST
                    || [HTMINBUTTON, HTMAXBUTTON, HTCLOSE, HTSYSMENU].contains(&(result as u32)))
            {
                return result;
            }
            if message == WM_NCHITTEST {
                // Caption rectangles are live screen pixels, independent of UI
                // zoom and client offsets introduced by maximize/restore.
                let screen = super::screen_point(lparam);
                let mut buttons = RECT::default();
                let mut outer = RECT::default();
                if DwmGetWindowAttribute(
                    handle,
                    DWMWA_CAPTION_BUTTON_BOUNDS as u32,
                    (&mut buttons as *mut RECT).cast(),
                    size_of::<RECT>() as u32,
                ) >= 0
                    && GetWindowRect(handle, &mut outer) != 0
                {
                    let bounds = [
                        outer.left + buttons.left,
                        outer.top + buttons.top,
                        outer.left + buttons.right,
                        outer.top + buttons.bottom,
                    ];
                    if let Some(index) = super::caption_button_index(screen, bounds) {
                        return [HTMINBUTTON, HTMAXBUTTON, HTCLOSE][index] as isize;
                    }
                }
                let native = DefSubclassProc(handle, message, wparam, lparam);
                if [HTMINBUTTON, HTMAXBUTTON, HTCLOSE, HTSYSMENU].contains(&(native as u32)) {
                    return native;
                }
                let mut rect = RECT::default();
                if GetWindowRect(handle, &mut rect) != 0 {
                    let state = &*(data as *const HeaderState);
                    let x = screen[0] - rect.left;
                    let y = screen[1] - rect.top;
                    use windows_sys::Win32::UI::HiDpi::{GetDpiForWindow, GetSystemMetricsForDpi};
                    let dpi = GetDpiForWindow(handle);
                    let dpi = if dpi == 0 { 96 } else { dpi };
                    let padding = GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi);
                    let edge_x = GetSystemMetricsForDpi(SM_CXSIZEFRAME, dpi) + padding;
                    let edge_y = GetSystemMetricsForDpi(SM_CYSIZEFRAME, dpi) + padding;
                    if IsZoomed(handle) == 0 {
                        let left = x < edge_x;
                        let right = x >= rect.right - rect.left - edge_x;
                        let top = y < edge_y;
                        let bottom = y >= rect.bottom - rect.top - edge_y;
                        let hit = match (left, right, top, bottom) {
                            (true, _, true, _) => HTTOPLEFT,
                            (_, true, true, _) => HTTOPRIGHT,
                            (true, _, _, true) => HTBOTTOMLEFT,
                            (_, true, _, true) => HTBOTTOMRIGHT,
                            (true, _, _, _) => HTLEFT,
                            (_, true, _, _) => HTRIGHT,
                            (_, _, true, _) => HTTOP,
                            (_, _, _, true) => HTBOTTOM,
                            _ => HTCLIENT,
                        };
                        if hit != HTCLIENT {
                            return hit as isize;
                        }
                    }
                    let mut client = windows_sys::Win32::Foundation::POINT {
                        x: screen[0],
                        y: screen[1],
                    };
                    windows_sys::Win32::Graphics::Gdi::ScreenToClient(handle, &mut client);
                    if super::in_drag_region(
                        client.x as f32 / state.scale,
                        client.y as f32 / state.scale,
                        &state.drag,
                    ) {
                        return HTCAPTION as isize;
                    }
                    return HTCLIENT as isize;
                }
            }
            DefSubclassProc(handle, message, wparam, lparam)
        }
    }
    pub fn install_header(window: &impl HasWindowHandle) -> Result<(), String> {
        use windows_sys::Win32::UI::{
            Shell::{GetWindowSubclass, RemoveWindowSubclass, SetWindowSubclass},
            WindowsAndMessaging::*,
        };
        let handle = hwnd(window)?;
        let mut existing = 0;
        // SAFETY: installation and all subsequent updates occur on the owning
        // UI thread. The registered Box is transferred to the window callback.
        unsafe {
            if GetWindowSubclass(handle, Some(header_proc), HEADER_ID, &mut existing) != 0 {
                return Ok(());
            }
            attribute(handle, DWMWA_NCRENDERING_POLICY, DWMNCRP_ENABLED)?;
            let data = Box::into_raw(Box::new(HeaderState {
                scale: 1.0,
                drag: Vec::new(),
            }));
            if SetWindowSubclass(handle, Some(header_proc), HEADER_ID, data as usize) == 0 {
                drop(Box::from_raw(data));
                return Err("Native header integration failed.".into());
            }
            if SetWindowPos(
                handle,
                std::ptr::null_mut(),
                0,
                0,
                0,
                0,
                SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            ) == 0
            {
                RemoveWindowSubclass(handle, Some(header_proc), HEADER_ID);
                drop(Box::from_raw(data));
                return Err("Native frame extension failed.".into());
            }
        }
        Ok(())
    }
    pub fn header_regions(window: &impl HasWindowHandle, scale: f32, regions: Vec<[f32; 4]>) {
        use windows_sys::Win32::UI::Shell::GetWindowSubclass;
        let Ok(handle) = hwnd(window) else {
            return;
        };
        let mut data = 0;
        // SAFETY: this is the same UI thread as the subclass; no window message
        // is dispatched while the state is borrowed, and the pointer is not retained.
        unsafe {
            if GetWindowSubclass(handle, Some(header_proc), HEADER_ID, &mut data) != 0 && data != 0
            {
                let state = &mut *(data as *mut HeaderState);
                state.scale = scale.max(0.5);
                state.drag = regions;
            }
        }
    }
    fn geometry(window: &impl HasWindowHandle) -> Result<(HWND, RECT, RECT), String> {
        let handle = hwnd(window)?;
        let mut rect = RECT::default();
        let mut monitor = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        // SAFETY: outputs are correctly sized writable structs, and the HWND is live.
        let valid = unsafe {
            GetWindowRect(handle, &mut rect) != 0
                && GetMonitorInfoW(
                    MonitorFromWindow(handle, MONITOR_DEFAULTTONEAREST),
                    &mut monitor,
                ) != 0
        };
        if !valid {
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok((handle, rect, monitor.rcWork))
    }
    pub fn center(window: &impl HasWindowHandle) -> Result<(), String> {
        let (handle, rect, work) = geometry(window)?;
        let (x, y) = centered_position(
            [rect.left, rect.top, rect.right, rect.bottom],
            [work.left, work.top, work.right, work.bottom],
        );
        // SAFETY: eframe owns the HWND; only its position changes. No borrowed memory is retained.
        if unsafe {
            SetWindowPos(
                handle,
                std::ptr::null_mut(),
                x,
                y,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            )
        } == 0
        {
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok(())
    }
    /// Verification-only snapshot of this app's visible compositor rectangle.
    pub fn capture(window: &impl HasWindowHandle, path: &std::path::Path) -> Result<(), String> {
        capture_handle(hwnd(window)?, path)
    }
    fn capture_handle(handle: HWND, path: &std::path::Path) -> Result<(), String> {
        use windows_sys::Win32::Graphics::Gdi::*;
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GetForegroundWindow, IsIconic, IsWindowVisible,
        };
        // Verification must never capture an unrelated overlapping window.
        // The app's framebuffer capture remains available in the background.
        if unsafe { GetForegroundWindow() } != handle {
            return Err("Compositor capture requires this app window in the foreground.".into());
        }
        let mut rect = RECT::default();
        // SAFETY: all handles belong to this live window, and DWM writes a sized RECT.
        let visible = unsafe {
            IsWindowVisible(handle) != 0
                && IsIconic(handle) == 0
                && DwmGetWindowAttribute(
                    handle,
                    DWMWA_EXTENDED_FRAME_BOUNDS as u32,
                    (&mut rect as *mut RECT).cast(),
                    size_of::<RECT>() as u32,
                ) >= 0
        };
        if !visible {
            return Err("The app window must be visible for a compositor capture.".into());
        }
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        if width <= 0 || height <= 0 || width as u64 * height as u64 > 16_777_216 {
            return Err("Invalid or oversized app-window capture bounds.".into());
        }
        let length = width as usize * height as usize * 4;
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                ..Default::default()
            },
            ..Default::default()
        };
        // SAFETY: capture is restricted to the rectangle supplied by DWM for our
        // own window. The checked top-down 32-bit DIB has exactly 'length' bytes;
        // every byte is initialized before GDI writes. DwmFlush/GdiFlush synchronize
        // access. Pixel data is copied before all owned GDI resources are released.
        let mut rgba = unsafe {
            if DwmFlush() < 0 {
                return Err("DWM could not synchronize the window capture.".into());
            }
            let screen = GetDC(std::ptr::null_mut());
            if screen.is_null() {
                return Err(std::io::Error::last_os_error().to_string());
            }
            let memory = CreateCompatibleDC(screen);
            if memory.is_null() {
                ReleaseDC(std::ptr::null_mut(), screen);
                return Err("Cannot create capture DC.".into());
            }
            let mut bits = std::ptr::null_mut();
            let bitmap = CreateDIBSection(
                screen,
                &info,
                DIB_RGB_COLORS,
                &mut bits,
                std::ptr::null_mut(),
                0,
            );
            if bitmap.is_null() || bits.is_null() {
                if !bitmap.is_null() {
                    DeleteObject(bitmap);
                }
                DeleteDC(memory);
                ReleaseDC(std::ptr::null_mut(), screen);
                return Err("Cannot allocate app-window capture bitmap.".into());
            }
            std::ptr::write_bytes(bits.cast::<u8>(), 0, length);
            let previous = SelectObject(memory, bitmap);
            let selected = !previous.is_null() && previous as isize != -1;
            let copied = selected
                && BitBlt(
                    memory,
                    0,
                    0,
                    width,
                    height,
                    screen,
                    rect.left,
                    rect.top,
                    SRCCOPY | CAPTUREBLT,
                ) != 0
                && GdiFlush() != 0;
            let pixels = if copied {
                Some(std::slice::from_raw_parts(bits.cast::<u8>(), length).to_vec())
            } else {
                None
            };
            if selected {
                SelectObject(memory, previous);
            }
            DeleteDC(memory);
            DeleteObject(bitmap);
            ReleaseDC(std::ptr::null_mut(), screen);
            pixels.ok_or_else(|| "Cannot read composited app-window pixels.".to_string())?
        };
        for pixel in rgba.as_chunks_mut::<4>().0 {
            pixel.swap(0, 2);
            pixel[3] = 255;
        }
        image::save_buffer_with_format(
            path,
            &rgba,
            width as u32,
            height as u32,
            image::ColorType::Rgba8,
            image::ImageFormat::Png,
        )
        .map_err(|e| e.to_string())
    }

    fn owned_named(title: &str) -> Result<HWND, String> {
        use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW, GetWindowThreadProcessId};
        let title: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
        // SAFETY: title is terminated UTF-16 and lives through the call. The HWND
        // is accepted only if its process is ours; operations run on the UI thread.
        unsafe {
            let handle = FindWindowW(std::ptr::null(), title.as_ptr());
            let mut pid = 0;
            if handle.is_null()
                || GetWindowThreadProcessId(handle, &mut pid) == 0
                || pid != std::process::id()
            {
                return Err("Owned guide window is not ready.".into());
            }
            Ok(handle)
        }
    }
    struct NamedWindow(HWND);
    impl HasWindowHandle for NamedWindow {
        fn window_handle(
            &self,
        ) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
            let native = raw_window_handle::Win32WindowHandle::new(
                std::num::NonZeroIsize::new(self.0 as isize).expect("validated HWND"),
            );
            // SAFETY: named lookup verified the app-owned HWND, and the UI callback
            // owns its lifetime during this synchronous configuration.
            unsafe {
                Ok(raw_window_handle::WindowHandle::borrow_raw(
                    RawWindowHandle::Win32(native),
                ))
            }
        }
    }
    pub fn owned_window_key(title: &str) -> Option<usize> {
        owned_named(title).ok().map(|h| h as usize)
    }
    pub fn configure_owned(
        title: &str,
        enabled: bool,
        backdrop: Backdrop,
        dark: bool,
    ) -> Result<(), String> {
        configure(&NamedWindow(owned_named(title)?), enabled, backdrop, dark)
    }
    pub fn capture_owned(title: &str, path: &std::path::Path) -> Result<(), String> {
        capture_handle(owned_named(title)?, path)
    }
    pub fn report_owned(title: &str) -> serde_json::Value {
        owned_named(title)
            .map(|h| report(&NamedWindow(h)))
            .unwrap_or(serde_json::Value::Null)
    }
    pub fn report(window: &impl HasWindowHandle) -> serde_json::Value {
        let Ok((handle, rect, work)) = geometry(window) else {
            return serde_json::Value::Null;
        };
        let mut material = 0_i32;
        let mut dark = 0_i32;
        let mut caption = 0_i32;
        let mut rendering = 0_i32;
        let mut buttons = RECT::default();
        let mut native_hits = Vec::new();
        // SAFETY: diagnostics run on the owning UI thread with a live HWND.
        unsafe {
            DwmGetWindowAttribute(
                handle,
                DWMWA_NCRENDERING_ENABLED as u32,
                (&mut rendering as *mut i32).cast(),
                4,
            );
            if DwmGetWindowAttribute(
                handle,
                DWMWA_CAPTION_BUTTON_BOUNDS as u32,
                (&mut buttons as *mut RECT).cast(),
                size_of::<RECT>() as u32,
            ) >= 0
                && buttons.right > buttons.left
            {
                for index in 0..3 {
                    let x = rect.left
                        + buttons.left
                        + (buttons.right - buttons.left) * (2 * index + 1) / 6;
                    let y = rect.top + buttons.top + (buttons.bottom - buttons.top) / 2;
                    let packed = (x as u16 as u32) | ((y as u16 as u32) << 16);
                    let mut hit = 0;
                    let handled = DwmDefWindowProc(
                        handle,
                        windows_sys::Win32::UI::WindowsAndMessaging::WM_NCHITTEST,
                        0,
                        packed as isize,
                        &mut hit,
                    ) != 0;
                    native_hits.push(serde_json::json!({"handled": handled, "hit": hit}));
                }
            }
        }
        // SAFETY: each DWM output is initialized, writable, and has the exact requested size.
        let (style, material_ok, dark_ok, caption_ok) = unsafe {
            (
                GetWindowLongPtrW(handle, GWL_STYLE) as u32,
                DwmGetWindowAttribute(
                    handle,
                    DWMWA_SYSTEMBACKDROP_TYPE as u32,
                    (&mut material as *mut i32).cast(),
                    4,
                ) >= 0,
                DwmGetWindowAttribute(
                    handle,
                    DWMWA_USE_IMMERSIVE_DARK_MODE as u32,
                    (&mut dark as *mut i32).cast(),
                    4,
                ) >= 0,
                DwmGetWindowAttribute(
                    handle,
                    DWMWA_CAPTION_COLOR as u32,
                    (&mut caption as *mut i32).cast(),
                    4,
                ) >= 0,
            )
        };
        let expected = centered_position(
            [rect.left, rect.top, rect.right, rect.bottom],
            [work.left, work.top, work.right, work.bottom],
        );
        serde_json::json!({"native_caption":style & WS_CAPTION == WS_CAPTION && rendering != 0,"caption_style":style & WS_CAPTION == WS_CAPTION,"minimize_button":style & windows_sys::Win32::UI::WindowsAndMessaging::WS_MINIMIZEBOX != 0,"maximize_button":style & windows_sys::Win32::UI::WindowsAndMessaging::WS_MAXIMIZEBOX != 0,"system_menu":style & windows_sys::Win32::UI::WindowsAndMessaging::WS_SYSMENU != 0,"dwm_caption_hits":native_hits,"backdrop_type":if material_ok {Some(material)}else{None},"dark_caption":if dark_ok {Some(dark!=0)}else{None},"caption_color":if caption_ok {Some(caption as u32)}else{None},"caption_controls_pixels":caption_controls_rect(window,1.0),"outer_rect":[rect.left,rect.top,rect.right,rect.bottom],"monitor_work_area":[work.left,work.top,work.right,work.bottom],"center_error_pixels":[rect.left-expected.0,rect.top-expected.1]})
    }
    use super::{caption_surface, centered_position};
}
#[cfg(target_os = "windows")]
pub use windows::{
    caption_controls_rect, capture, capture_owned, center, configure, configure_owned,
    header_regions, header_top_inset, install_header, owned_window_key, report, report_owned,
    splash_frame,
};

/// Keep Acrylic's title bar clear for DWM's backdrop and native button layer.
pub fn paint_header_background(
    ctx: &eframe::egui::Context,
    frame: &eframe::Frame,
    painter: &eframe::egui::Painter,
    color: eframe::egui::Color32,
    acrylic: bool,
) {
    if cfg!(target_os = "windows") && acrylic {
        // A client tint over this strip also tints DWM's native glyphs. Let
        // Windows composite the material and buttons without app overdraw.
        return;
    }
    let rect = header_content_rect(ctx, frame);
    painter.with_clip_rect(rect).rect_filled(rect, 0.0, color);
}

/// The app-drawn region excludes caption controls at every UI scale.
pub fn header_content_rect(
    ctx: &eframe::egui::Context,
    frame: &eframe::Frame,
) -> eframe::egui::Rect {
    #[cfg(target_os = "windows")]
    let right = caption_controls_rect(frame, ctx.pixels_per_point()).map_or(
        (ctx.screen_rect().right() - 138.0).max(0.0),
        |bounds| {
            if bounds[3] > 0.0 {
                bounds[0].max(0.0)
            } else {
                ctx.screen_rect().right()
            }
        },
    );
    #[cfg(not(target_os = "windows"))]
    let right = {
        let _ = frame;
        ctx.screen_rect().right()
    };
    let inset = header_top_inset(frame, ctx.pixels_per_point());
    eframe::egui::Rect::from_min_max(
        eframe::egui::pos2(0.0, inset),
        eframe::egui::pos2(right, 38.0 + inset),
    )
}

#[cfg(not(target_os = "windows"))]
pub fn header_top_inset(_window: &eframe::Frame, _scale: f32) -> f32 {
    0.0
}

#[cfg(not(target_os = "windows"))]
pub fn splash_frame(_window: &eframe::Frame) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "windows")]
fn caption_surface(dark: bool) -> [u8; 3] {
    if dark { [29, 32, 40] } else { [247, 249, 253] }
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
pub fn configure(
    _window: &eframe::Frame,
    _enabled: bool,
    _backdrop: Backdrop,
    _dark: bool,
) -> Result<(), String> {
    Err("Native Windows backdrops are unavailable on this platform.".into())
}
#[cfg(not(target_os = "windows"))]
pub fn center(_window: &eframe::Frame) -> Result<(), String> {
    Ok(())
}
#[cfg(not(target_os = "windows"))]
pub fn capture(_window: &eframe::Frame, _path: &std::path::Path) -> Result<(), String> {
    Err("Native compositor capture is available on Windows.".into())
}
#[cfg(not(target_os = "windows"))]
pub fn report(_window: &eframe::Frame) -> serde_json::Value {
    serde_json::Value::Null
}

#[cfg(any(target_os = "windows", test))]
fn centered_position(rect: [i32; 4], work: [i32; 4]) -> (i32, i32) {
    let width = rect[2] - rect[0];
    let height = rect[3] - rect[1];
    (
        work[0] + ((work[2] - work[0] - width) / 2).max(0),
        work[1] + ((work[3] - work[1] - height) / 2).max(0),
    )
}
#[cfg(test)]
mod tests {
    #[test]
    fn center_uses_work_area_offsets_and_negative_monitor_origins() {
        assert_eq!(
            super::centered_position([0, 0, 1200, 800], [0, 0, 1920, 1040]),
            (360, 120)
        );
        assert_eq!(
            super::centered_position([0, 0, 1200, 800], [-1920, 40, 0, 1080]),
            (-1560, 160)
        );
        assert_eq!(
            super::centered_position([0, 0, 2200, 1400], [0, 0, 1920, 1040]),
            (0, 0)
        );
    }
}

#[cfg(not(target_os = "windows"))]
pub fn header_regions(_window: &eframe::Frame, _scale: f32, _regions: Vec<[f32; 4]>) {}

#[cfg(any(target_os = "windows", test))]
fn in_drag_region(x: f32, y: f32, regions: &[[f32; 4]]) -> bool {
    regions
        .iter()
        .any(|r| x >= r[0] && x < r[2] && y >= r[1] && y < r[3])
}

#[cfg(any(target_os = "windows", test))]
fn screen_point(packed: isize) -> [i32; 2] {
    [
        (packed as u32 as u16 as i16) as i32,
        ((packed as u32 >> 16) as u16 as i16) as i32,
    ]
}

#[cfg(any(target_os = "windows", test))]
fn contains_screen_point(point: [i32; 2], rect: [i32; 4]) -> bool {
    rect[0] < rect[2]
        && rect[1] < rect[3]
        && point[0] >= rect[0]
        && point[0] < rect[2]
        && point[1] >= rect[1]
        && point[1] < rect[3]
}

#[cfg(test)]
mod header_tests {
    #[test]
    fn native_button_groups_ignore_ui_zoom_and_maximized_offsets() {
        for dpi_scale in [1.0, 1.25, 1.5, 2.0] {
            let width = (138.0 * dpi_scale) as i32;
            for origin in [[0, 0], [-1920, -8], [2422, 0]] {
                let bounds = [
                    origin[0],
                    origin[1],
                    origin[0] + width,
                    origin[1] + (30.0 * dpi_scale) as i32,
                ];
                for index in 0..3 {
                    let x = origin[0] + ((index * 2 + 1) as i32 * width) / 6;
                    assert_eq!(
                        super::caption_button_index([x, origin[1] + 10], bounds),
                        Some(index)
                    );
                }
                assert_eq!(
                    super::caption_button_index([bounds[2], origin[1]], bounds),
                    None
                );
            }
        }
    }
    #[test]
    fn caption_pixels_handle_negative_monitors_and_scaled_rectangles() {
        for scale in [1.0, 1.25, 1.5, 2.0] {
            let left = -1920;
            let rect = [
                left,
                -8,
                left + (46.0 * scale) as i32,
                -8 + (30.0 * scale) as i32,
            ];
            let point = [(rect[0] + rect[2]) / 2, (rect[1] + rect[3]) / 2];
            let packed = (point[0] as u16 as u32 | ((point[1] as u16 as u32) << 16)) as isize;
            assert_eq!(super::screen_point(packed), point);
            assert!(super::contains_screen_point(point, rect));
            assert!(!super::contains_screen_point([rect[2], point[1]], rect));
            assert!(!super::contains_screen_point(point, [0; 4]));
        }
    }
    #[test]
    fn header_drag_excludes_command_and_caption_buttons() {
        let regions = [[170.0, 6.0, 850.0, 38.0]];
        assert!(super::in_drag_region(400.0, 20.0, &regions));
        assert!(!super::in_drag_region(90.0, 20.0, &regions));
        assert!(!super::in_drag_region(1040.0, 20.0, &regions));
        assert!(!super::in_drag_region(400.0, 55.0, &regions));
    }
}

#[cfg(any(target_os = "windows", test))]
fn caption_button_index(point: [i32; 2], bounds: [i32; 4]) -> Option<usize> {
    if !contains_screen_point(point, bounds) {
        return None;
    }
    let width = i64::from(bounds[2]) - i64::from(bounds[0]);
    Some(((i64::from(point[0]) - i64::from(bounds[0])) * 3 / width) as usize)
}

#[cfg(not(target_os = "windows"))]
pub fn owned_window_key(_title: &str) -> Option<usize> {
    None
}
#[cfg(not(target_os = "windows"))]
pub fn configure_owned(
    _title: &str,
    _enabled: bool,
    _backdrop: Backdrop,
    _dark: bool,
) -> Result<(), String> {
    Ok(())
}
#[cfg(not(target_os = "windows"))]
pub fn capture_owned(_title: &str, _path: &std::path::Path) -> Result<(), String> {
    Err("Native guide capture is available on Windows.".into())
}

#[cfg(not(target_os = "windows"))]
pub fn report_owned(_title: &str) -> serde_json::Value {
    serde_json::Value::Null
}

#[cfg(target_os = "macos")]
#[path = "macos_backdrop.rs"]
mod macos_backdrop;
#[cfg(target_os = "macos")]
pub use macos_backdrop::configure;
#[cfg(target_os = "linux")]
#[path = "linux_backdrop.rs"]
mod linux_backdrop;
#[cfg(target_os = "linux")]
pub use linux_backdrop::{configure, shutdown};
#[cfg(not(target_os = "linux"))]
pub fn shutdown() {}
