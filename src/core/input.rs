use std::{cell::RefCell, rc::Rc};

use smithay::{
    backend::input::{
        AbsolutePositionEvent, ButtonState, Event, InputBackend, InputEvent, KeyboardKeyEvent,
        PointerButtonEvent,
    },
    input::{
        keyboard::FilterResult,
        pointer::{ButtonEvent, MotionEvent},
    },
    utils::SERIAL_COUNTER,
};

use crate::{
    core::Hazel,
    lua::api::{
        wm_input_keyboard::KeyboardEvent,
        wm_input_pointer::{LuaPointerButtonEvent, LuaPointerMotionEvent},
    },
};

impl Hazel {
    pub fn process_input<B: InputBackend>(
        &mut self,
        event: InputEvent<B>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.wm().input.events.emit("event".to_owned(), ())?;

        match event {
            InputEvent::Keyboard { event } => {
                let keycode = event.key_code();
                let state = event.state();
                let serial = SERIAL_COUNTER.next_serial();
                let time = event.time_msec();

                let keyboard = self.compositor.seat.get_keyboard().unwrap();
                keyboard.input::<(), _>(
                    self,
                    keycode,
                    state,
                    serial,
                    time,
                    move |hazel, modifiers, kh| {
                        let event = Rc::new(KeyboardEvent {
                            keycode,
                            keysyms: kh.modified_syms(),
                            modifiers: modifiers.clone(),
                            state,
                            serial,
                            time,
                            default_prevented: RefCell::new(false),
                        });

                        hazel
                            .wm()
                            .input
                            .events
                            .emit("keyboard".to_owned(), event.clone())
                            .expect("Oops");

                        if event.default_prevented.take() {
                            println!("Default prevented for key event: {:?}", event.keycode);
                            FilterResult::Intercept(())
                        } else {
                            FilterResult::Forward
                        }
                    },
                );
            }

            InputEvent::PointerMotionAbsolute { event } => {
                let pointer = self.compositor.seat.get_pointer().unwrap();

                let output = self.compositor.space.outputs().next().unwrap();

                let geo = self.compositor.space.output_geometry(output).unwrap();
                let location = event.position_transformed(geo.size) + geo.loc.to_f64();
                let under = self.compositor.surface_under(location);

                let event = LuaPointerMotionEvent {
                    event: MotionEvent {
                        serial: SERIAL_COUNTER.next_serial(),
                        time: event.time_msec(),
                        location,
                    },
                    default_prevented: RefCell::new(false),
                };

                self.wm()
                    .input
                    .events
                    .emit(LuaPointerMotionEvent::name(), event.clone())
                    .expect("Failed to emit pointer move event");

                if !event.default_prevented.into_inner() {
                    pointer.motion(self, under, &event.event);
                    pointer.frame(self);
                }
            }

            InputEvent::PointerButton { event } => {
                let event = ButtonEvent {
                    button: event.button_code(),
                    state: event.state(),
                    serial: SERIAL_COUNTER.next_serial(),
                    time: event.time_msec(),
                };

                if event.state == ButtonState::Pressed {
                    self.pointer_pressed.push(event.button);
                } else {
                    self.pointer_pressed.retain(|&b| b != event.button);
                }

                let keyboard = self.compositor.seat.get_keyboard().unwrap();
                let pointer = self.compositor.seat.get_pointer().unwrap();

                self.wm()
                    .input
                    .events
                    .emit(
                        LuaPointerButtonEvent::name(),
                        LuaPointerButtonEvent(event.clone()),
                    )
                    .expect("Failed to emit pointer button event");

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
                    } else {
                        self.compositor.space.elements().for_each(|window| {
                            window.set_activated(false);
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
}
