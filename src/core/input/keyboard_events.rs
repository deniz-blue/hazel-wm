use smithay::{
    backend::input::{Device, Event, InputBackend, KeyboardKeyEvent},
    input::keyboard::FilterResult,
    utils::SERIAL_COUNTER,
};

use crate::{core::Hazel, lua::api::wm_input_keyboard::KeyboardEvent};

impl Hazel {
    pub fn on_keyboard_key<B: InputBackend>(&mut self, event: B::KeyboardKeyEvent) {
        let device_id = event.device().id();
        let keycode = event.key_code();
        let state = event.state();
        let serial = SERIAL_COUNTER.next_serial();
        let time = event.time_msec();

        let (_, keyboard) = self.compositor.get_keyboard_handle(&device_id).unwrap();

        keyboard.input::<(), _>(
            self,
            keycode,
            state,
            serial,
            time,
            move |hazel, modifiers, kh| {
                let event = KeyboardEvent {
                    keycode,
                    keysym: kh.modified_sym(),
                    keysyms: kh.modified_syms(),
                    modifiers: modifiers.clone(),
                    state,
                    serial,
                    time,
                    default_prevented: Default::default(),
                };

                hazel
                    .wm()
                    .input
                    .events
                    .emit(KeyboardEvent::name(), event.clone())
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
}
