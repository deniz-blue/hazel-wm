use smithay::backend::{input::InputEvent, winit::WinitInput};

use crate::core::Hazel;

impl Hazel {
    pub fn process_input(&mut self, _event: InputEvent<WinitInput>) {
        // println!("Input event: {:?}", event);

        let _ = self
            .lua
            .wm()
            .input
            .events
            .emit(&self.lua.lua, "event".to_owned(), ());
    }

	pub fn doohickey(&mut self) {
		println!("Doohickey called!!!!!!!!");
	}
}
