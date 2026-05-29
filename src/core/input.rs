use smithay::backend::{input::InputEvent, winit::WinitInput};

use crate::core::Hazel;

impl Hazel {
    pub fn process_input(&mut self, _event: InputEvent<WinitInput>) {
        if let Err(e) = self.wm().input.events.emit("event".to_owned(), ()) {
            eprintln!("Error emitting Lua input event: {e}");
        }
    }

    pub fn doohickey(&mut self) {
        println!("Doohickey called!!!!!!!!");
    }
}
