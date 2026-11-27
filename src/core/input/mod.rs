use smithay::backend::input::{InputBackend, InputEvent};
use std::error::Error as StdError;

use crate::core::{Hazel, input::gestures::GestureKind};

pub mod device_events;
pub mod gestures;
pub mod keyboard_events;
pub mod pointer;
pub mod pointer_constraints;
pub mod pointer_events;
pub mod pointer_grab;
pub mod scroll;

impl Hazel {
    pub fn process_input<B: InputBackend>(
        &mut self,
        event: InputEvent<B>,
    ) -> std::result::Result<(), Box<dyn StdError>> {
        match event {
            InputEvent::DeviceAdded { device } => self.on_device_added(device),
            InputEvent::DeviceRemoved { device } => self.on_device_removed(device),
            InputEvent::Keyboard { event } => self.on_keyboard_key::<B>(event),

            InputEvent::PointerMotion { event } => self.on_pointer_motion::<B>(event),
            InputEvent::PointerMotionAbsolute { event } => self.on_pointer_absolute::<B>(event),

            InputEvent::PointerAxis { event } => self.on_pointer_axis::<B>(event),

            InputEvent::PointerButton { event } => self.on_pointer_button::<B>(event),

            InputEvent::GestureSwipeBegin { event } => Ok(self.on_gesture_begin::<_, B>(GestureKind::Swipe, &event)),
            InputEvent::GestureSwipeUpdate { event } => Ok(self.on_swipe_update::<B>(&event)),
            InputEvent::GestureSwipeEnd { event } => Ok(self.on_gesture_end::<_, B>(GestureKind::Swipe, &event)),
            InputEvent::GesturePinchBegin { event } => Ok(self.on_gesture_begin::<_, B>(GestureKind::Pinch, &event)),
            InputEvent::GesturePinchUpdate { event } => Ok(self.on_pinch_update::<B>(&event)),
            InputEvent::GesturePinchEnd { event } => Ok(self.on_gesture_end::<_, B>(GestureKind::Pinch, &event)),
            InputEvent::GestureHoldBegin { event } => Ok(self.on_gesture_begin::<_, B>(GestureKind::Hold, &event)),
            InputEvent::GestureHoldEnd { event } => Ok(self.on_gesture_end::<_, B>(GestureKind::Hold, &event)),

            _ => Ok(()),
        }
    }
}
