use mlua::UserData;

use crate::{impl_lua_event_handler, impl_lua_event_source, lua::event_handler::LuaEventHandler};

#[derive(Default)]
pub struct WmInput {
    pub events: LuaEventHandler,
}

impl_lua_event_source!(WmInput);

impl UserData for WmInput {
    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        impl_lua_event_handler!(methods);
    }
}
