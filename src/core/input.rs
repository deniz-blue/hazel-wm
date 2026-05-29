use smithay::{
    backend::{
        input::{
            AbsolutePositionEvent, ButtonState, Event, InputEvent, KeyboardKeyEvent,
            PointerButtonEvent,
        },
        winit::WinitInput,
    },
    input::{
        keyboard::FilterResult,
        pointer::{ButtonEvent, MotionEvent},
    },
    utils::SERIAL_COUNTER,
};

use crate::core::Hazel;

impl Hazel {
    pub fn process_input(
        &mut self,
        event: InputEvent<WinitInput>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.wm().input.events.emit("event".to_owned(), ())?;

        match event {
            InputEvent::Keyboard { event } => {
                let serial = SERIAL_COUNTER.next_serial();
                let time = event.time_msec();
                let keycode = event.key_code();
                let state = event.state();

				println!("Keyboard event: keycode={}, state={:?}", keycode.raw(), state);

                let keyboard = self.compositor.seat.get_keyboard().unwrap();
                keyboard.input::<(), _>(self, keycode, state, serial, time, |_, _, _| {
                    FilterResult::Forward
                });
            }

            InputEvent::PointerMotionAbsolute { event } => {
                let pointer = self.compositor.seat.get_pointer().unwrap();

                let output = self.compositor.space.outputs().next().unwrap();

                let geo = self.compositor.space.output_geometry(output).unwrap();
                let location = event.position_transformed(geo.size) + geo.loc.to_f64();
                let under = self.compositor.surface_under(location);

                let event = MotionEvent {
                    serial: SERIAL_COUNTER.next_serial(),
                    time: event.time_msec(),
                    location,
                };

                pointer.motion(self, under, &event);
                pointer.frame(self);
            }

            InputEvent::PointerButton { event } => {
                let event = ButtonEvent {
                    button: event.button_code(),
                    state: event.state(),
                    serial: SERIAL_COUNTER.next_serial(),
                    time: event.time_msec(),
                };

                let keyboard = self.compositor.seat.get_keyboard().unwrap();
                let pointer = self.compositor.seat.get_pointer().unwrap();

                if ButtonState::Pressed == event.state && !pointer.is_grabbed() {
                    if let Some((window, _)) = self
                        .compositor
                        .space
                        .element_under(pointer.current_location())
                        .map(|(w, l)| (w.clone(), l))
                    {
                        self.compositor.space.raise_element(&window, true);
                        keyboard.set_focus(
                            self,
                            Some(window.toplevel().unwrap().wl_surface().clone()),
                            event.serial,
                        );
                        self.compositor.space.elements().for_each(|window| {
                            window.toplevel().unwrap().send_pending_configure();
                        });
                    } else {
                        self.compositor.space.elements().for_each(|window| {
                            window.set_activated(false);
                            window.toplevel().unwrap().send_pending_configure();
                        });
                        keyboard.set_focus(self, None, event.serial);
                    }
                };

                pointer.button(self, &event);
                pointer.frame(self);
            }

            _ => {}
        }

        Ok(())
    }

    pub fn doohickey(&mut self) {
        println!("Doohickey called!!!!!!!!");
    }
}
