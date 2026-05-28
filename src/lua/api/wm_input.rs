use mlua::{Function, UserData};

use crate::{impl_lua_event_handler, lua::event_handler::{LuaEventHandler, LuaEventSource}};

#[derive(Default)]
pub struct WmInput {
    pub events: LuaEventHandler,
}

impl LuaEventSource for WmInput {
	fn events(&self) -> &LuaEventHandler {
		&self.events
	}
}

impl UserData for WmInput {
	fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
		impl_lua_event_handler!(methods);
	}
}

