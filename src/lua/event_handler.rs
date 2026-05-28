use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use mlua::{Function, IntoLua, RegistryKey, UserData};

#[derive(Clone, Default)]
pub struct LuaEventHandler {
    handlers: Arc<Mutex<HashMap<String, Vec<RegistryKey>>>>,
}

impl LuaEventHandler {
    pub fn register(
        &self,
        lua: &mlua::Lua,
        event_name: String,
        handler: Function,
    ) -> Result<(), mlua::Error> {
        let key = lua.create_registry_value(handler)?;
        self.handlers
            .lock()
            .unwrap()
            .entry(event_name)
            .or_default()
            .push(key);
        Ok(())
    }

    pub fn emit<D: IntoLua>(&self, lua: &mlua::Lua, event_name: &str, data: D) -> Result<(), mlua::Error> {
        self.emit_with_hook(lua, event_name, data, || Ok(()))
    }

    pub fn emit_with_hook<D: IntoLua, H: FnMut() -> Result<(), mlua::Error>>(
        &self,
        lua: &mlua::Lua,
        event_name: &str,
        data: D,
        mut after_each: H,
    ) -> Result<(), mlua::Error> {
        let data_lua = data.into_lua(lua)?;

        if let Some(handlers) = self.handlers.lock().unwrap().get(event_name) {
            for handler in handlers {
                let func = lua.registry_value::<Function>(handler)?;
                if let Err(err) = func.call::<()>(data_lua.clone()) {
                    eprintln!("Lua handler error for '{event_name}': {err}");
                }

                after_each()?;
            }
        }

        Ok(())
    }
}

impl UserData for LuaEventHandler {
    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut(
            "on",
            |lua, this, (event_name, handler): (String, Function)| {
                this.register(lua, event_name, handler)
            },
        );
    }
}
