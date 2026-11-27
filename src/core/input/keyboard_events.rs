use smithay::{
    backend::input::{Device, Event, InputBackend, KeyState, KeyboardKeyEvent, Keycode},
    input::keyboard::{FilterResult, KeyboardHandle, keysyms},
    utils::SERIAL_COUNTER,
};
use std::error::Error as StdError;
use tracing::{error, info, warn};

use crate::core::compositor::KeyRepeat;
use crate::{core::Hazel, lua::api::wm_input_keyboard::KeyEvent};

impl Hazel {
    pub fn on_keyboard_key<B: InputBackend>(
        &mut self,
        event: B::KeyboardKeyEvent,
    ) -> std::result::Result<(), Box<dyn StdError>> {
        let device_id = event.device().id();
        let Some((_, keyboard)) = self.compositor.get_keyboard_handle(&device_id) else {
            warn!("Keyboard event from device {device_id} that no seat has");
            return Ok(());
        };

        self.press_key(&keyboard, event.key_code(), event.state(), event.time_msec())
    }

    pub fn press_key(
        &mut self,
        keyboard: &KeyboardHandle<Hazel>,
        keycode: Keycode,
        state: KeyState,
        time: u32,
    ) -> std::result::Result<(), Box<dyn StdError>> {
        self.notify_idle_activity();
        let serial = SERIAL_COUNTER.next_serial();
        let keyboard = keyboard.clone();
        let event_keyboard = keyboard.clone();

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
                    raw_keysym: kh
                        .raw_latin_sym_or_raw_current_sym()
                        .unwrap_or_else(|| kh.modified_sym()),
                    modifiers: *modifiers,
                    state,
                    serial,
                    time,
                    default_prevented: Default::default(),
                    shortcuts_inhibited: hazel.compositor.shortcuts_inhibited(&event_keyboard),
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

                if let Err(e) = hazel.wm().input.events.emit(KeyEvent::name(), event.clone()) {
                    error!("Key event could not reach Lua: {e}");
                }

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

impl Hazel {
    pub fn set_key_repeat(&mut self, rate: i32, delay: i32) {
        self.compositor.key_repeat = KeyRepeat { rate, delay };
        for seat in self.compositor.seats.values() {
            if let Some(keyboard) = seat.get_keyboard() {
                keyboard.change_repeat_info(rate, delay);
            }
        }
    }
}
