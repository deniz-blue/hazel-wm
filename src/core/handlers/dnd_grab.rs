use smithay::{
    input::{Seat, pointer::Focus},
    input::dnd::{DnDGrab, DndGrabHandler, DndTarget, GrabType, Source},
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::{Logical, Point, Serial},
    wayland::selection::data_device::WaylandDndGrabHandler,
};

use crate::core::Hazel;

impl WaylandDndGrabHandler for Hazel {
    fn dnd_requested<S: Source>(
        &mut self,
        source: S,
        _icon: Option<WlSurface>,
        seat: Seat<Self>,
        serial: Serial,
        type_: GrabType,
    ) {
        match type_ {
            GrabType::Pointer => {
                let Some(pointer) = seat.get_pointer() else { return };
                let Some(start_data) = pointer.grab_start_data() else { return };
                let grab = DnDGrab::new_pointer(&self.display_handle, start_data, source, seat);
                pointer.set_grab(self, grab, serial, Focus::Keep);
            }
            GrabType::Touch => {
                let Some(touch) = seat.get_touch() else { return };
                let Some(start_data) = touch.grab_start_data() else { return };
                let grab = DnDGrab::new_touch(&self.display_handle, start_data, source, seat);
                touch.set_grab(self, grab, serial);
            }
        }
    }
}

impl DndGrabHandler for Hazel {
    fn dropped(&mut self, _target: Option<DndTarget<'_, Self>>, _validated: bool, _seat: Seat<Self>, _location: Point<f64, Logical>) {}
}
