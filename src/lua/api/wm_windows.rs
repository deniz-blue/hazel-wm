use mlua::UserData;

use crate::{impl_lua_event_handler, impl_lua_event_source, lua::event_handler::LuaEventHandler};

#[derive(Default)]
pub struct WmWindows {
    pub events: LuaEventHandler,
}

impl_lua_event_source!(WmWindows);

impl UserData for WmWindows {
    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        impl_lua_event_handler!(methods);
    }
}
