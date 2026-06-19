use crate::{
    backend::{Backend, tty::HazelTtyBackend, winit::HazelWinitBackend},
    core::Hazel,
};

impl Hazel {
    pub fn backend_post_init(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        match self.backend {
            Backend::Winit(HazelWinitBackend { ref output }) => {
                let _global = output.create_global::<Hazel>(&self.display_handle);
                self.compositor.space.map_output(&output, (0, 0));
            }
			Backend::Udev(HazelTtyBackend { .. }) => {

			}
        };

        Ok(())
    }
}
