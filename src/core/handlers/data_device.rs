use smithay::wayland::selection::data_device::{DataDeviceHandler, DataDeviceState};

use crate::core::Hazel;

impl DataDeviceHandler for Hazel {
    fn data_device_state(&mut self) -> &mut DataDeviceState {
        &mut self.compositor.smithay.data_device_state
    }
}

