use smithay::{
    delegate_seat,
    input::{
        SeatHandler, SeatState,
        pointer::CursorImageStatus,
    },
    reexports::wayland_server::protocol::wl_surface::WlSurface,
};

use crate::core::{Hazel, input::pointer::PointerState};

impl SeatHandler for Hazel {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;
    type TouchFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.compositor.smithay.seat_state
    }

    fn cursor_image(&mut self, seat: &SmithaySeat<Self>, image: CursorImageStatus) {
        tracing::trace!(?image, "cursor image updated");
        if let Some(pointer) = seat.get_pointer() {
            self.compositor
                .pointers
                .entry(pointer)
                .or_insert_with(PointerState::new)
                .cursor_status = image;
        }
    }
}

// alias to avoid conflict with Hazel's Seat struct
use smithay::input::Seat as SmithaySeat;

delegate_seat!(Hazel);
