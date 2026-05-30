use mlua::UserData;

use crate::{
    impl_lua_event_handler, impl_lua_event_source,
    lua::{
        api::{wm_input_keyboard::WmInputKeyboard, wm_input_pointer::WmInputPointer},
        event_handler::LuaEventHandler,
    },
};

#[derive(Default)]
pub struct WmInput {
    pub events: LuaEventHandler,
}

impl_lua_event_source!(WmInput);

impl UserData for WmInput {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("keyboard", |_, _| Ok(WmInputKeyboard));
        fields.add_field_method_get("pointer", |_, _| Ok(WmInputPointer));
    }

    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        impl_lua_event_handler!(methods);
    }
}
