use smithay::{
    input::pointer::PointerHandle,
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::{Logical, Point},
    wayland::pointer_constraints::PointerConstraintsHandler,
};

use crate::{
    core::Hazel,
    lua::api::wm_input_constraints::WmPointerConstraint,
};

impl PointerConstraintsHandler for Hazel {
    fn new_constraint(&mut self, surface: &WlSurface, pointer: &PointerHandle<Self>) {
        let _ = self.wm().input.events.emit(
            WmPointerConstraint::name(),
            WmPointerConstraint { surface: surface.clone(), pointer: pointer.clone() },
        );
    }

    fn cursor_position_hint(&mut self, _: &WlSurface, _: &PointerHandle<Self>, _: Point<f64, Logical>) {}
}

