use std::{cell::RefCell, rc::Rc};

use mlua::UserData;
use smithay::{
    backend::input::{KeyState, Keycode},
    input::keyboard::{KeyboardHandle, Keysym, ModifiersState, XkbConfig},
    utils::Serial,
};
use tracing::warn;

use crate::{
    core::{GlobalHazel, Hazel, locking::LockRecovering},
    lua::api::{wm_input_seats::WmSeat, wm_input_sym::LuaKeysym},
    lua_typedef,
};

const EVDEV_TO_XKB_KEYCODE_OFFSET: u32 = 8;

pub struct WmInputKeyboard(pub KeyboardHandle<Hazel>);

impl UserData for WmInputKeyboard {
    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("seat", |_, this, _: ()| {
            GlobalHazel::try_with(|hazel| {
                Ok(hazel
                    .compositor
                    .seats
                    .values()
                    .find(|seat| seat.get_keyboard().map(|k| k == this.0).unwrap_or(false))
                    .map(|s| WmSeat(s.clone())))
            })
        });

        methods.add_method("key", |_, this, (evdev_code, state): (u32, String)| {
            let state = match state.as_str() {
                "pressed" | "Pressed" => KeyState::Pressed,
                "released" | "Released" => KeyState::Released,
                other => {
                    return Err(mlua::Error::external(format!(
                        "key state must be \"pressed\" or \"released\", got {other:?}"
                    )));
                }
            };
            GlobalHazel::try_with(|hazel| {
                hazel
                    .press_key(&this.0, Keycode::new(evdev_code + EVDEV_TO_XKB_KEYCODE_OFFSET), state, 0)
                    .map_err(|e| mlua::Error::external(e.to_string()))
            })
        });

        methods.add_method("get_layout", |_, this, _: ()| {
            GlobalHazel::try_with(|hazel| {
                Ok(this.0.with_xkb_state(hazel, |k| {
                    let state = k.xkb().lock_recovering();
                    state.layout_name(state.active_layout()).to_owned()
                }))
            })
        });

        methods.add_method("repeat_rate", |_, _, _: ()| {
            GlobalHazel::try_with(|hazel| Ok(hazel.compositor.key_repeat.rate))
        });

        methods.add_method("repeat_delay", |_, _, _: ()| {
            GlobalHazel::try_with(|hazel| Ok(hazel.compositor.key_repeat.delay))
        });

        methods.add_method("set_repeat", |_, _, (rate, delay): (i32, i32)| {
            if rate < 0 || delay < 0 {
                return Err(mlua::Error::external(
                    "repeat rate and delay must not be negative",
                ));
            }
            GlobalHazel::try_with(|hazel| {
                hazel.set_key_repeat(rate, delay);
                Ok(())
            })
        });

        methods.add_method("set_layout", |_, this, keymap: String| {
            if keymap.contains('\0') {
                return Err(mlua::Error::external("a keymap name cannot contain a NUL byte"));
            }
            GlobalHazel::try_with(|hazel| {
                let result = this.0.set_xkb_config(
                    hazel,
                    XkbConfig {
                        layout: &keymap,
                        ..XkbConfig::default()
                    },
                );

                if let Err(e) = &result {
                    warn!("Failed to set keyboard layout to \"{}\": {}", keymap, e);
                }

                Ok(result.is_ok())
            })
        });
    }
}

lua_typedef!(Keyboard => WmInputKeyboard {
    fn seat() -> Option<Seat>;
    fn get_layout() -> string;
    fn set_layout(keymap: string) -> boolean;
    fn repeat_rate() -> integer;
    fn repeat_delay() -> integer;
    fn set_repeat(rate: integer, delay: integer) -> nil;
});

#[derive(Clone, Debug)]
pub struct KeyEvent {
    pub keyboard: KeyboardHandle<Hazel>,
    pub keycode: Keycode,
    pub keysym: Keysym,
    pub keysyms: Vec<Keysym>,
    pub raw_keysym: Keysym,
    pub modifiers: ModifiersState,
    pub state: KeyState,
    pub serial: Serial,
    pub time: u32,
    pub default_prevented: Rc<RefCell<bool>>,
    pub shortcuts_inhibited: bool,
}

impl KeyEvent {
    pub fn name() -> String {
        String::from("key")
    }

    pub fn prevent_default(&self) {
        self.default_prevented.replace(true);
    }

    pub fn is_default_prevented(&self) -> bool {
        *self.default_prevented.borrow()
    }
}

impl UserData for KeyEvent {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("keyboard", |_, this| {
            Ok(WmInputKeyboard(this.keyboard.clone()))
        });
        fields.add_field_method_get("shortcuts_inhibited", |_, this| Ok(this.shortcuts_inhibited));
        fields.add_field_method_get("state", |_, this| Ok(format!("{:?}", this.state)));
        fields.add_field_method_get("serial", |_, this| Ok(Into::<u32>::into(this.serial)));
        fields.add_field_method_get("time", |_, this| Ok(this.time));
        fields.add_field_method_get("keycode", |_, this| Ok(this.keycode.raw()));
        fields.add_field_method_get("key", |_, this| Ok(LuaKeysym(this.keysym)));
        fields.add_field_method_get("raw_key", |_, this| Ok(LuaKeysym(this.raw_keysym)));
        fields.add_field_method_get("keys", |_, this| {
            Ok(this
                .keysyms
                .iter()
                .map(|ks| LuaKeysym(*ks))
                .collect::<Vec<_>>())
        });
        fields.add_field_method_get("modifiers", |_, this| {
            Ok(ModifiersStateUserData(this.modifiers))
        });
    }

    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("prevent_default", |_, this, ()| {
            this.default_prevented.replace(true);
            Ok(())
        });
    }
}

lua_typedef!(KeyEvent => KeyEvent {
    let keyboard: Keyboard;
    let state: string;
    let serial: number;
    let time: number;
    let keycode: number;
    let key: Keysym;
    let raw_key: Keysym;
    let keys: Array<Keysym>;
    let modifiers: Modifiers;
    let shortcuts_inhibited: boolean;
    fn prevent_default() -> nil;
});

pub struct ModifiersStateUserData(pub ModifiersState);

impl UserData for ModifiersStateUserData {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("shift", |_, this| Ok(this.0.shift));
        fields.add_field_method_get("ctrl", |_, this| Ok(this.0.ctrl));
        fields.add_field_method_get("alt", |_, this| Ok(this.0.alt));
        fields.add_field_method_get("logo", |_, this| Ok(this.0.logo));
        fields.add_field_method_get("caps_lock", |_, this| Ok(this.0.caps_lock));
        fields.add_field_method_get("num_lock", |_, this| Ok(this.0.num_lock));
        fields.add_field_method_get("altgr", |_, this| Ok(this.0.iso_level3_shift));
    }
}

lua_typedef!(Modifiers => ModifiersStateUserData {
    let shift: boolean;
    let ctrl: boolean;
    let alt: boolean;
    let logo: boolean;
    let caps_lock: boolean;
    let num_lock: boolean;
    let altgr: boolean;
});
