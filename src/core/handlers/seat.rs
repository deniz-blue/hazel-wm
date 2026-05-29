use smithay::{
    delegate_seat, input::{SeatHandler, SeatState}, reexports::wayland_server::protocol::wl_surface::WlSurface
};

use crate::core::Hazel;

impl SeatHandler for Hazel {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;
    type TouchFocus = WlSurface;
    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.compositor.smithay.seat_state
    }
}

delegate_seat!(Hazel);
