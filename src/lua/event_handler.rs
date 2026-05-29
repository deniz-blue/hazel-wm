use std::{cell::RefCell, collections::HashMap};

use mlua::{Function, IntoLuaMulti, RegistryKey, UserData};

use crate::core::GlobalHazel;

#[derive(Default)]
pub struct LuaEventHandler {
    handlers: RefCell<HashMap<String, Vec<RegistryKey>>>,
}

impl LuaEventHandler {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add(&self, event_name: String, handler: RegistryKey) -> Result<(), mlua::Error> {
        self.handlers
            .borrow_mut()
            .entry(event_name)
            .or_default()
            .push(handler);
        Ok(())
    }

    pub fn add_function(
        &self,
        lua: &mlua::Lua,
        event_name: String,
        handler: Function,
    ) -> Result<(), mlua::Error> {
        let registry_key = lua.create_registry_value(handler)?;
        self.add(event_name, registry_key)
    }

    pub fn emit_with<A: IntoLuaMulti + Clone>(
        &self,
        lua: &mlua::Lua,
        event_name: String,
        args: A,
    ) -> Result<(), mlua::Error> {
        if let Some(handlers) = self.handlers.borrow_mut().get(&event_name) {
            for handler in handlers {
                if let Ok(handler_fn) = lua.registry_value::<Function>(handler) {
                    let _ = handler_fn.call::<mlua::Value>(args.clone());
                }
            }
        }
        Ok(())
    }

    pub fn emit<A: IntoLuaMulti + Clone>(
        &self,
        event_name: String,
        args: A,
    ) -> Result<(), mlua::Error> {
		GlobalHazel::with(|hazel| {
			self.emit_with(&hazel.lua.lua, event_name, args)
		})
    }
}

pub trait LuaEventSource {
    fn events(&self) -> &LuaEventHandler;
}

impl UserData for LuaEventHandler {
    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method(
            "on",
            |lua, this, (event_name, handler): (String, Function)| {
                this.add_function(lua, event_name, handler)
            },
        );
    }
}

#[macro_export]
macro_rules! impl_lua_event_handler {
    ($methods:ident) => {
        $methods.add_method(
            "on",
            |lua: &mlua::Lua, this, (event_name, handler): (String, Function)| {
                this.events().add_function(lua, event_name, handler)
            },
        );
    };
}
