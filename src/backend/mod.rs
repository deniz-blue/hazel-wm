use smithay::{backend::session::Session, reexports::wayland_server::DisplayHandle};

use crate::{
    backend::{tty::HazelTtyBackend, winit::HazelWinitBackend},
    core::HazelEventLoop,
};

#[derive(Debug)]
pub enum Backend {
    Winit(HazelWinitBackend),
    Udev(HazelTtyBackend),
}

pub mod tty;
pub mod winit;

impl Backend {
    pub fn create_winit(
        event_loop: &mut HazelEventLoop,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self::Winit(HazelWinitBackend::new(event_loop)?))
    }

    pub fn create_tty(
        display_handle: &DisplayHandle,
        event_loop: &mut HazelEventLoop,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self::Udev(HazelTtyBackend::new(
            display_handle,
            event_loop,
        )?))
    }

    pub fn switch_vt(&mut self, vt: u32) -> Result<(), Box<dyn std::error::Error>> {
        if let Self::Udev(backend) = self {
            backend.session.change_vt(vt as i32)?
        }

        Ok(())
    }

    pub fn as_tty(&mut self) -> &mut HazelTtyBackend {
        match self {
            Self::Udev(backend) => backend,
            _ => panic!("Not a TTY backend"),
        }
    }
}
