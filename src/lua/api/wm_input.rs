use mlua::{Function, UserData};

use crate::{
    impl_lua_event_handler,
    lua::{
        HazelHandle,
        event_handler::{LuaEventHandler, LuaEventSource},
    },
};

pub struct WmInput {
    pub hazel: HazelHandle,
    pub events: LuaEventHandler,
}

impl WmInput {
    pub fn new(h: HazelHandle) -> Self {
        Self {
            hazel: h,
            events: LuaEventHandler::new(),
        }
    }
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
