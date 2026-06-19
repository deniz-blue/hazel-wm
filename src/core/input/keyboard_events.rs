use smithay::{
    backend::input::{Device, Event, InputBackend, KeyboardKeyEvent},
    input::keyboard::{FilterResult, keysyms},
    utils::SERIAL_COUNTER,
};
use tracing::{error, info};
use std::error::Error as StdError;

use crate::{core::Hazel, lua::api::wm_input_keyboard::KeyEvent};

impl Hazel {
    pub fn on_keyboard_key<B: InputBackend>(
        &mut self,
        event: B::KeyboardKeyEvent,
    ) -> std::result::Result<(), Box<dyn StdError>> {
        let device_id = event.device().id();
        let keycode = event.key_code();
        let state = event.state();
        let serial = SERIAL_COUNTER.next_serial();
        let time = event.time_msec();

        let (_, keyboard) = self.compositor.get_keyboard_handle(&device_id).unwrap();

        keyboard.clone().input::<(), _>(
            self,
            keycode,
            state,
            serial,
            time,
            move |hazel, modifiers, kh| {
                let event = KeyEvent {
                    keyboard,
                    keycode,
                    keysym: kh.modified_sym(),
                    keysyms: kh.modified_syms(),
                    modifiers: modifiers.clone(),
                    state,
                    serial,
                    time,
                    default_prevented: Default::default(),
                };

                if (keysyms::KEY_XF86Switch_VT_1..=keysyms::KEY_XF86Switch_VT_12)
                    .contains(&event.keysym.raw())
                {
                    let vt = event.keysym.raw() - keysyms::KEY_XF86Switch_VT_1 + 1;
                    info!("Switching to VT {}", vt);
                    if let Err(e) = hazel.backend.switch_vt(vt) {
                        error!("Failed to switch VT: {e}");
                    }
                }

                hazel
                    .wm()
                    .input
                    .events
                    .emit(KeyEvent::name(), event.clone())
                    .expect("Oops");

                if event.default_prevented.take() {
                    info!("Default prevented for key event: {:?}", event.keycode);
                    FilterResult::Intercept(())
                } else {
                    FilterResult::Forward
                }
            },
        );

        Ok(())
    }
}
