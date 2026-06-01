use std::error::Error as StdError;
use smithay::{
    backend::input::{Device, DeviceCapability},
    input::keyboard::XkbConfig,
};

use crate::core::Hazel;

impl Hazel {
    pub fn on_device_added(&mut self, device: impl Device) -> std::result::Result<(), Box<dyn StdError>> {
        let id = device.id();
        println!("Device added: {id}");

        let seat_to_assign = String::from("seat0");
        let seat = self
            .compositor
            .seats
            .entry(seat_to_assign.clone())
            .or_insert_with(|| {
                println!("Creating new seat: {seat_to_assign}");
                self.compositor
                    .smithay
                    .seat_state
                    .new_wl_seat(&self.display_handle, seat_to_assign.clone())
            });

        if device.has_capability(DeviceCapability::Keyboard) {
            seat.add_keyboard(XkbConfig::default(), 200, 25).unwrap();
        }

        if device.has_capability(DeviceCapability::Pointer) {
            seat.add_pointer();
        }

        println!("Assigning device {id} to seat {seat_to_assign}");
        self.compositor.device_to_seat.insert(id, seat_to_assign);
        Ok(())
    }

    pub fn on_device_removed(&mut self, device: impl Device) -> std::result::Result<(), Box<dyn StdError>> {
        let id = device.id();
        println!("Device removed: {id}");

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
            println!("Removing device {id} from seat {seat_name}");
            if device.has_capability(DeviceCapability::Keyboard) {
                seat.remove_keyboard();
            }
            if device.has_capability(DeviceCapability::Pointer) {
                seat.remove_pointer();
            }
        } else {
            println!("Device {id} was not assigned to any seat");
        }

        self.compositor.device_to_seat.remove(&id);
        Ok(())
    }
}
