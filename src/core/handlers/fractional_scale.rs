use smithay::{
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    wayland::{
        compositor::with_states,
        fractional_scale::{FractionalScaleHandler, with_fractional_scale},
    },
};

use crate::core::Hazel;

impl FractionalScaleHandler for Hazel {
    fn new_fractional_scale(&mut self, surface: WlSurface) {
        let scale = self.compositor.preferred_scale_for(&surface);
        with_states(&surface, |states| {
            with_fractional_scale(states, |state| state.set_preferred_scale(scale));
        });
    }
}

