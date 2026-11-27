use smithay::{
    wayland::selection::{
        ext_data_control::{DataControlHandler as ExtDataControlHandler, DataControlState as ExtDataControlState},
        wlr_data_control::{DataControlHandler as WlrDataControlHandler, DataControlState as WlrDataControlState},
    },
};

use crate::core::Hazel;

impl WlrDataControlHandler for Hazel {
    fn data_control_state(&mut self) -> &mut WlrDataControlState {
        &mut self.compositor.smithay.wlr_data_control_state
    }
}

impl ExtDataControlHandler for Hazel {
    fn data_control_state(&mut self) -> &mut ExtDataControlState {
        &mut self.compositor.smithay.ext_data_control_state
    }
}

