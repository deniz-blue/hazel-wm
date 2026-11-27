use smithay::{
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    wayland::xdg_system_bell::XdgSystemBellHandler,
};

use crate::{core::Hazel, lua::api::wm_windows::WmWindow};

impl XdgSystemBellHandler for Hazel {
    fn ring(&mut self, surface: Option<WlSurface>) {
        let window = surface.and_then(|surface| self.compositor.window_of_surface(&surface));
        let _ = self
            .wm()
            .windows
            .events
            .emit("bell".to_owned(), window.map(WmWindow));
    }
}

