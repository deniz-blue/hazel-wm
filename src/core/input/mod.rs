use smithay::backend::input::{InputBackend, InputEvent};

use crate::core::Hazel;

pub mod device_events;
pub mod keyboard_events;
pub mod pointer;
pub mod pointer_events;

impl Hazel {
    pub fn process_input<B: InputBackend>(
        &mut self,
        event: InputEvent<B>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match event {
            InputEvent::DeviceAdded { device } => self.on_device_added(device),
            InputEvent::DeviceRemoved { device } => self.on_device_removed(device),
            InputEvent::Keyboard { event } => self.on_keyboard_key::<B>(event),

            InputEvent::PointerMotion { event } => self.on_pointer_motion::<B>(event),
            InputEvent::PointerMotionAbsolute { event } => self.on_pointer_absolute::<B>(event),

            InputEvent::PointerAxis { event } => self.on_pointer_axis::<B>(event),

            InputEvent::PointerButton { event } => self.on_pointer_button::<B>(event),

            _ => {}
        }

        Ok(())
    }
}
