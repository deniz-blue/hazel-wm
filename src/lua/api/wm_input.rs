use mlua::UserData;

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
