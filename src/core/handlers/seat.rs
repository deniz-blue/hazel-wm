use smithay::{
    input::{
        Seat as SmithaySeat, SeatHandler, SeatState,
        pointer::CursorImageStatus,
        tablet::TabletSeatHandler,
    },
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    wayland::{
        selection::{data_device::set_data_device_focus, primary_selection::set_primary_focus},
    },
};

use crate::core::{Hazel, input::pointer::PointerState};

impl SeatHandler for Hazel {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;
    type TouchFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.compositor.smithay.seat_state
    }

    fn focus_changed(&mut self, seat: &SmithaySeat<Self>, focused: Option<&WlSurface>) {
        self.apply_keyboard_focus(seat, focused);
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

impl TabletSeatHandler for Hazel {
    type ToolFocus = WlSurface;
}


impl Hazel {
    pub fn apply_keyboard_focus(&mut self, seat: &SmithaySeat<Self>, focused: Option<&WlSurface>) {
        let client = focused.and_then(|surface| self.client_of(surface));
        set_data_device_focus(&self.display_handle, seat, client.clone());
        set_primary_focus(&self.display_handle, seat, client);

        let window = focused.and_then(|surface| self.compositor.window_of_surface(surface));
        self.compositor.pending_focus_changes.push((seat.name().to_owned(), window));
    }
}
