//! Desktop blur negotiation for Wayland and X11. Unsupported compositors use
//! the same themed surfaces without transparency.
#![allow(unsafe_code)]
use adars::settings::Backdrop;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};
use std::cell::RefCell;
use wayland_client::protocol::{wl_compositor, wl_region, wl_registry, wl_surface};
use wayland_client::{Connection, Dispatch, EventQueue, Proxy, QueueHandle, WEnum, delegate_noop};
use wayland_protocols::ext::background_effect::v1::client::{
    ext_background_effect_manager_v1 as effect_manager,
    ext_background_effect_surface_v1 as effect_surface,
};
use wayland_protocols_plasma::blur::client::{
    org_kde_kwin_blur as kde_blur, org_kde_kwin_blur_manager as kde_manager,
};

thread_local! {
    static WAYLAND: RefCell<Option<WaylandBlur>> = const { RefCell::new(None) };
}

pub fn configure(
    window: &eframe::Frame,
    enabled: bool,
    _material: Backdrop,
    _dark: bool,
) -> Result<(), String> {
    let handle = window.window_handle().map_err(|e| e.to_string())?.as_raw();
    match handle {
        RawWindowHandle::Xlib(h) => x11(
            h.window.try_into().map_err(|_| "Invalid X11 window")?,
            enabled,
        ),
        RawWindowHandle::Xcb(h) => x11(h.window.get(), enabled),
        RawWindowHandle::Wayland(h) => {
            let RawDisplayHandle::Wayland(display) =
                window.display_handle().map_err(|e| e.to_string())?.as_raw()
            else {
                return Err("No Wayland display".into());
            };
            WAYLAND.with(|slot| {
                let mut slot = slot.borrow_mut();
                if slot.is_none() {
                    if !enabled {
                        return Ok(());
                    }
                    // SAFETY: the guest backend borrows eframe's live display and
                    // surface on its UI thread. shutdown drops it in App::on_exit,
                    // before eframe destroys the display. We never own the surface.
                    *slot = Some(unsafe {
                        WaylandBlur::new(display.display.as_ptr(), h.surface.as_ptr())?
                    });
                }
                slot.as_mut().expect("initialized").set(enabled)
            })
        }
        _ => Err("Desktop blur unavailable".into()),
    }
}

pub fn shutdown() {
    WAYLAND.with(|slot| {
        if let Some(mut blur) = slot.borrow_mut().take() {
            let _ = blur.set(false);
            if let Some(effect) = blur.state.effect.take() {
                effect.destroy();
            }
            if let Some(effect) = blur.state.kde_effect.take() {
                effect.release();
            }
            if let Some(manager) = blur.state.manager.take() {
                manager.destroy();
            }
            let _ = blur.connection.flush();
        }
    });
}

fn x11(window: u32, enabled: bool) -> Result<(), String> {
    use x11rb::{
        connection::Connection as _,
        protocol::xproto::{AtomEnum, ConnectionExt as _, PropMode},
        wrapper::ConnectionExt as _,
    };
    let (connection, screen) = x11rb::connect(None).map_err(|e| e.to_string())?;
    let root = connection.setup().roots[screen].root;
    let atom = connection
        .intern_atom(!enabled, b"_KDE_NET_WM_BLUR_BEHIND_REGION")
        .map_err(|e| e.to_string())?
        .reply()
        .map_err(|e| e.to_string())?
        .atom;
    if atom == 0 {
        return if enabled {
            Err("Desktop blur unavailable".into())
        } else {
            Ok(())
        };
    }
    if enabled {
        // EWMH advertises effect support independently of the desktop name.
        let net_supported = connection
            .intern_atom(true, b"_NET_SUPPORTED")
            .map_err(|e| e.to_string())?
            .reply()
            .map_err(|e| e.to_string())?
            .atom;
        if net_supported == 0 {
            return Err("Desktop blur unavailable".into());
        }
        let properties = connection
            .get_property(false, root, net_supported, AtomEnum::ATOM, 0, 16384)
            .map_err(|e| e.to_string())?
            .reply()
            .map_err(|e| e.to_string())?;
        if !properties
            .value32()
            .is_some_and(|mut values| values.any(|v| v == atom))
        {
            return Err("Desktop blur unavailable".into());
        }
        connection
            .change_property32(PropMode::REPLACE, window, atom, AtomEnum::CARDINAL, &[])
            .map_err(|e| e.to_string())?
            .check()
            .map_err(|e| e.to_string())?;
    } else {
        connection
            .delete_property(window, atom)
            .map_err(|e| e.to_string())?
            .check()
            .map_err(|e| e.to_string())?;
    }
    connection.flush().map_err(|e| e.to_string())
}

#[derive(Default)]
struct State {
    compositor: Option<wl_compositor::WlCompositor>,
    manager: Option<effect_manager::ExtBackgroundEffectManagerV1>,
    kde_manager: Option<kde_manager::OrgKdeKwinBlurManager>,
    effect: Option<effect_surface::ExtBackgroundEffectSurfaceV1>,
    kde_effect: Option<kde_blur::OrgKdeKwinBlur>,
    supports_blur: bool,
}
struct WaylandBlur {
    connection: Connection,
    queue: EventQueue<State>,
    _registry: wl_registry::WlRegistry,
    surface: wl_surface::WlSurface,
    state: State,
}
impl WaylandBlur {
    unsafe fn new(
        display: *mut std::ffi::c_void,
        surface: *mut std::ffi::c_void,
    ) -> Result<Self, String> {
        // SAFETY: caller guarantees both pointers stay alive until shutdown.
        let backend =
            unsafe { wayland_client::backend::Backend::from_foreign_display(display.cast()) };
        let connection = Connection::from_backend(backend);
        let id = unsafe {
            wayland_client::backend::ObjectId::from_ptr(
                wl_surface::WlSurface::interface(),
                surface.cast(),
            )
        }
        .map_err(|e| e.to_string())?;
        let surface = wl_surface::WlSurface::from_id(&connection, id).map_err(|e| e.to_string())?;
        let mut queue = connection.new_event_queue();
        let registry = connection.display().get_registry(&queue.handle(), ());
        let mut state = State::default();
        queue.roundtrip(&mut state).map_err(|e| e.to_string())?;
        // A second round trip obtains capabilities from any newly bound manager.
        queue.roundtrip(&mut state).map_err(|e| e.to_string())?;
        Ok(Self {
            connection,
            queue,
            _registry: registry,
            surface,
            state,
        })
    }
    fn set(&mut self, enabled: bool) -> Result<(), String> {
        self.queue
            .dispatch_pending(&mut self.state)
            .map_err(|e| e.to_string())?;
        let qh = self.queue.handle();
        if self.state.supports_blur {
            let manager = self
                .state
                .manager
                .as_ref()
                .ok_or("Desktop blur unavailable")?;
            let effect = self
                .state
                .effect
                .get_or_insert_with(|| manager.get_background_effect(&self.surface, &qh, ()));
            if enabled {
                let compositor = self
                    .state
                    .compositor
                    .as_ref()
                    .ok_or("Desktop blur unavailable")?;
                let region = compositor.create_region(&qh, ());
                // The compositor clips this region to the window after every resize.
                region.add(0, 0, i32::MAX, i32::MAX);
                effect.set_blur_region(Some(&region));
                region.destroy();
            } else {
                effect.set_blur_region(None);
            }
        } else if let Some(manager) = &self.state.kde_manager {
            if enabled {
                let effect = self
                    .state
                    .kde_effect
                    .get_or_insert_with(|| manager.create(&self.surface, &qh, ()));
                effect.set_region(None);
                effect.commit();
            } else {
                manager.unset(&self.surface);
                if let Some(effect) = self.state.kde_effect.take() {
                    effect.release();
                }
            }
        } else if enabled {
            return Err("Desktop blur unavailable".into());
        }
        // Surface commit belongs to eframe: committing here could conflict with
        // its configure/resize handshake. This state joins the next painted frame.
        self.connection.flush().map_err(|e| e.to_string())
    }
}
impl Dispatch<wl_registry::WlRegistry, ()> for State {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            match interface.as_str() {
                "wl_compositor" => {
                    state.compositor = Some(registry.bind(name, version.min(4), qh, ()))
                }
                "ext_background_effect_manager_v1" => {
                    state.manager = Some(registry.bind(name, 1, qh, ()))
                }
                "org_kde_kwin_blur_manager" => {
                    state.kde_manager = Some(registry.bind(name, 1, qh, ()))
                }
                _ => {}
            }
        }
    }
}
impl Dispatch<effect_manager::ExtBackgroundEffectManagerV1, ()> for State {
    fn event(
        state: &mut Self,
        _: &effect_manager::ExtBackgroundEffectManagerV1,
        event: effect_manager::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let effect_manager::Event::Capabilities { flags } = event {
            state.supports_blur = matches!(flags, WEnum::Value(value) if value.contains(effect_manager::Capability::Blur));
        }
    }
}
delegate_noop!(State: ignore wl_compositor::WlCompositor);
delegate_noop!(State: ignore wl_region::WlRegion);
delegate_noop!(State: ignore effect_surface::ExtBackgroundEffectSurfaceV1);
delegate_noop!(State: ignore kde_manager::OrgKdeKwinBlurManager);
delegate_noop!(State: ignore kde_blur::OrgKdeKwinBlur);
