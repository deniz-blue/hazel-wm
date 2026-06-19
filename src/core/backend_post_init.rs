use crate::{
    backend::{Backend, winit::HazelWinitBackend},
    core::Hazel,
};

impl Hazel {
    pub fn backend_post_init(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        match self.backend {
            Backend::Winit(HazelWinitBackend { ref output }) => {
                let _global = output.create_global::<Hazel>(&self.display_handle);
                self.compositor.space.map_output(&output, (0, 0));
            }
            Backend::Udev(ref mut backend) => {
                for device in backend.output_devices.values() {
                    for surface in device.surfaces.values() {
                        let _global = surface.output.create_global::<Hazel>(&self.display_handle);
                        self.compositor.space.map_output(&surface.output, (0, 0));
                    }
                }
            }
        };

        Ok(())
    }
}
