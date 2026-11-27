use smithay::{
    backend::session::Session,
    output::{Mode, Output},
    reexports::wayland_server::DisplayHandle,
    wayland::dmabuf::DmabufState,
};

use crate::{
    backend::{tty::HazelTtyBackend, winit::HazelWinitBackend},
    core::{HazelEventLoop, capture::target::DmabufCaptureSupport},
};

#[derive(Debug)]
pub enum Backend {
    Winit(HazelWinitBackend),
    Udev(HazelTtyBackend),
}

pub mod tty;
pub mod winit;
mod winit_capture;

#[derive(Debug, PartialEq, Eq)]
pub enum ModeChange {
    Applied,
    Requested,
}

impl Backend {
    pub fn create_winit(
        display_handle: &DisplayHandle,
        dmabuf_state: &mut DmabufState,
        event_loop: &mut HazelEventLoop,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self::Winit(HazelWinitBackend::new(display_handle, dmabuf_state, event_loop)?))
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

    pub fn request_output_mode(&mut self, output: &Output, mode: Mode) -> Result<ModeChange, String> {
        match self {
            Self::Winit(backend) => backend.request_mode(mode),
            Self::Udev(backend) => backend.use_mode(output, mode),
        }
    }

    pub fn as_tty(&mut self) -> Option<&mut HazelTtyBackend> {
        match self {
            Self::Udev(backend) => Some(backend),
            _ => None,
        }
    }
}

impl Backend {
    pub fn supports_screencopy(&self) -> bool {
        matches!(self, Self::Winit(_))
    }

    pub fn dmabuf_capture_support(&self) -> Option<&DmabufCaptureSupport> {
        match self {
            Self::Winit(backend) => backend.dmabuf_capture.as_ref(),
            Self::Udev(_) => None,
        }
    }
}
