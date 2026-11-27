use smithay::output::Output;

use crate::{
    backend::{Backend, winit::HazelWinitBackend},
    core::Hazel,
};

impl Hazel {
    pub fn backend_post_init(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        match self.backend {
            Backend::Winit(HazelWinitBackend { ref output, .. }) => {
                let output = output.clone();
                self.register_output(&output, Some((0, 0).into()));
            }
            Backend::Udev(ref backend) => {
                let outputs: Vec<Output> = backend
                    .output_devices
                    .values()
                    .flat_map(|device| device.surfaces.values().map(|surface| surface.output.clone()))
                    .collect();
                for output in &outputs {
                    self.register_output(output, None);
                }
            }
        };

        Ok(())
    }
}
