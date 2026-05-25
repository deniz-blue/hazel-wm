use std::{collections::HashMap, sync::{Arc, Mutex}};

use mlua::{IntoLua, UserData};

#[derive(Clone)]
pub struct LuaEventHandler {
    pub handlers: Arc<Mutex<HashMap<String, Vec<mlua::Function>>>>,
}

impl LuaEventHandler {
    pub fn new() -> Self {
        Self {
            handlers: Default::default(),
        }
    }

    pub fn emit<D: IntoLua>(
        &self,
        lua: &mlua::Lua,
        event_name: &str,
        data: D,
    ) -> Result<(), mlua::Error> {
        let data_lua = data.into_lua(lua)?;
        if let Some(handlers) = self.handlers.lock().unwrap().get(event_name) {
            for handler in handlers {
                handler.call::<()>(&data_lua)?;
				println!("Called Lua handler for event '{}'", event_name);
            }
        }

        Ok(())
    }
}

impl UserData for LuaEventHandler {
    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("on", |_, this, value: (String, mlua::Function)| {
            let (event_name, handler) = value;
			println!("Registering Lua handler for event '{event_name}'");
            this.handlers
                .lock()
                .unwrap()
                .entry(event_name)
                .or_insert_with(Vec::new)
                .push(handler);
            Ok(())
        });
    }
}

