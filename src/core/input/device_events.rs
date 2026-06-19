use smithay::{
    backend::input::{Device, DeviceCapability},
    input::keyboard::XkbConfig,
};
use std::error::Error as StdError;
use tracing::info;

use crate::{
    core::{Hazel, input::pointer::PointerState},
    lua::api::{wm_input_keyboard::WmInputKeyboard, wm_input_pointer::WmInputPointer},
};

impl Hazel {
    pub fn on_device_added(
        &mut self,
        device: impl Device,
    ) -> std::result::Result<(), Box<dyn StdError>> {
        let id = device.id();
        info!("Device added: {id}");

        let seat_to_assign = String::from("seat0");

        let mut seat = if let Some(seat) = self.compositor.seats.get(&seat_to_assign) {
            seat.clone()
        } else {
            info!("Creating new seat: {seat_to_assign}");
            let seat = self
                .compositor
                .smithay
                .seat_state
                .new_wl_seat(&self.display_handle, seat_to_assign.clone());
            self.compositor
                .seats
                .insert(seat_to_assign.clone(), seat.clone());
            seat
        };

        if device.has_capability(DeviceCapability::Keyboard) {
            info!("Adding keyboard capability to seat {seat_to_assign}");
            let keyboard = seat.add_keyboard(XkbConfig::default(), 200, 25).unwrap();
            self.wm()
                .input
                .events
                .emit(String::from("new_keyboard"), WmInputKeyboard(keyboard))?;
        }

        if device.has_capability(DeviceCapability::Pointer) {
            info!("Adding pointer capability to seat {seat_to_assign}");
            let pointer = seat.add_pointer();
            self.compositor
                .pointers
                .insert(pointer.clone(), PointerState::new());
            self.wm()
                .input
                .events
                .emit(String::from("new_pointer"), WmInputPointer(pointer))?;
        }

        info!("Assigning device {id} to seat {seat_to_assign}");
        self.compositor.device_to_seat.insert(id, seat_to_assign);
        Ok(())
    }

    pub fn on_device_removed(
        &mut self,
        device: impl Device,
    ) -> std::result::Result<(), Box<dyn StdError>> {
        let id = device.id();
        info!("Device removed: {id}");

        if let Some((seat_name, seat)) =
            self.compositor
                .device_to_seat
                .get(&id)
                .and_then(|seat_name| {
                    self.compositor
                        .seats
                        .get_mut(seat_name)
                        .map(|seat| (seat_name, seat))
                })
        {
            info!("Removing device {id} from seat {seat_name}");
            if device.has_capability(DeviceCapability::Keyboard) {
                seat.remove_keyboard();
            }
            if device.has_capability(DeviceCapability::Pointer) {
                if let Some(pointer) = seat.get_pointer() {
                    self.compositor.pointers.remove(&pointer);
                }
                seat.remove_pointer();
            }
        } else {
            info!("Device {id} was not assigned to any seat");
        }

        self.compositor.device_to_seat.remove(&id);

        self.compositor.seats.retain(|seat_name, seat| {
            let has_keyboard = seat.get_keyboard().is_some();
            let has_pointer = seat.get_pointer().is_some();
            let has_touch = seat.get_touch().is_some();
            if !has_keyboard && !has_pointer && !has_touch {
                info!("Removing empty seat: {seat_name}");
                false
            } else {
                true
            }
        });

        Ok(())
    }
}
