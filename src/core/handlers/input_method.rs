use smithay::{
    desktop::{PopupKind, PopupManager},
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::{Logical, Rectangle},
    wayland::input_method::{InputMethodHandler, PopupSurface},
};
use tracing::warn;

use crate::core::Hazel;

impl InputMethodHandler for Hazel {
    fn new_popup(&mut self, surface: PopupSurface) {
        if let Err(e) = self.compositor.smithay.popups.track_popup(PopupKind::from(surface)) {
            warn!("Could not track the input method popup: {e}");
        }
    }

    fn dismiss_popup(&mut self, surface: PopupSurface) {
        if let Some(parent) = surface.get_parent().map(|parent| parent.surface.clone()) {
            let _ = PopupManager::dismiss_popup(&parent, &PopupKind::from(surface));
        }
    }

    fn popup_repositioned(&mut self, _: PopupSurface) {}

    fn parent_geometry(&self, parent: &WlSurface) -> Rectangle<i32, Logical> {
        self.compositor
            .window_with_root_surface(parent)
            .map(|window| window.geometry())
            .unwrap_or_default()
    }
}

