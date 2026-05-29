use std::cell::RefCell;

use mlua::UserData;
use smithay::{
    backend::input::{KeyState, Keycode},
    input::keyboard::{Keysym, ModifiersState},
    utils::Serial,
};

use crate::{
    core::GlobalHazel, impl_lua_event_handler, impl_lua_event_source,
    lua::event_handler::LuaEventHandler,
};

#[derive(Default)]
pub struct WmInput {
    pub events: LuaEventHandler,
}

impl_lua_event_source!(WmInput);

impl UserData for WmInput {
    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        impl_lua_event_handler!(methods);

        methods.add_method("set_keyboard_layout", |_, _, keymap: String| {
            GlobalHazel::with(|hazel| {
                hazel
                    .compositor
                    .seat
                    .get_keyboard()
                    .unwrap()
                    .set_keymap_from_string(hazel, keymap)
                    .map_err(|e| mlua::Error::external(e))?;
                Ok(())
            })
        });
    }
}

pub struct KeyboardEvent {
    pub keycode: Keycode,
    pub keysyms: Vec<Keysym>,
    pub modifiers: ModifiersState,
    pub state: KeyState,
    pub serial: Serial,
    pub time: u32,
    pub default_prevented: RefCell<bool>,
}

impl UserData for KeyboardEvent {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("state", |_, this| Ok(format!("{:?}", this.state)));
        fields.add_field_method_get("serial", |_, this| Ok(Into::<u32>::into(this.serial)));
        fields.add_field_method_get("time", |_, this| Ok(this.time));
        fields.add_field_method_get("keycode", |_, this| Ok(this.keycode.raw()));
        fields.add_field_method_get("keysyms", |_, this| {
            let keysyms: Vec<_> = this.keysyms.iter().map(|ks| ks.raw()).collect();
            Ok(keysyms)
        });
        fields.add_field_method_get("modifiers", |_, this| {
            Ok(ModifiersStateUserData(this.modifiers.clone()))
        });
    }

    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("prevent_default", |_, this, ()| {
            this.default_prevented.replace(true);
            Ok(())
        });
    }
}

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
