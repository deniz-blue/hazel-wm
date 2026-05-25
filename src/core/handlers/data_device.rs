use smithay::wayland::selection::data_device::{DataDeviceHandler, DataDeviceState};

use crate::core::Hazel;

impl DataDeviceHandler for Hazel {
    fn data_device_state(&self) -> &DataDeviceState {
        &self.smithay.data_device_state
    }
}

smithay::delegate_data_device!(Hazel);
