use smithay::{desktop::layer_map_for_output, output::Output};
use tracing::info;

use crate::core::Hazel;

impl Hazel {
    pub fn on_output_added(&mut self, output: &Output) -> Result<(), Box<dyn std::error::Error>> {
        info!("Output added: {}", output.name());

        // Arrange layer surfaces for the new output.
        layer_map_for_output(output).arrange();

        self.wm()
            .outputs
            .events
            .emit("added".to_owned(), Option::<bool>::None)?;

        Ok(())
    }
}
