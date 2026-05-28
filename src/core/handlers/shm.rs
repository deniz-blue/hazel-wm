use smithay::{
    reexports::wayland_server::protocol::wl_buffer::WlBuffer,
    wayland::{
        buffer::BufferHandler,
        shm::{ShmHandler, ShmState},
    },
};

use crate::core::Hazel;

impl BufferHandler for Hazel {
    fn buffer_destroyed(&mut self, _buffer: &WlBuffer) {}
}

impl ShmHandler for Hazel {
    fn shm_state(&self) -> &ShmState {
        &self.compositor.smithay.shm_state
    }
}

smithay::delegate_shm!(Hazel);
