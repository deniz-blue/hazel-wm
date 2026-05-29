use smithay::output::Output;

use crate::core::Hazel;

impl Hazel {
    pub fn on_output_added(&mut self, output: &Output) -> Result<(), Box<dyn std::error::Error>> {
        println!("Output added: {}", output.name());

        self.wm()
            .outputs
            .events
            .emit("added".to_owned(), Option::<bool>::None)?;

        Ok(())
    }
}
