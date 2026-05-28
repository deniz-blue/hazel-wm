use std::sync::{Arc, Mutex};

use mlua::Function;

use crate::lua::event_handler::LuaEventHandler;

#[derive(Clone, Default)]
pub struct WmInput {
    events: LuaEventHandler,
    pending_layout: Arc<Mutex<Option<String>>>,
}

impl WmInput {
    pub fn register(
        &self,
        lua: &mlua::Lua,
        event_name: String,
        handler: Function,
    ) -> Result<(), mlua::Error> {
        self.events.register(lua, event_name, handler)
    }

    pub fn emit_keypress(&self, lua: &mlua::Lua, event: mlua::Table) -> Result<(), mlua::Error> {
        self.events.emit_with_hook(lua, "keypress", event, || Ok(()))
    }

    pub fn emit_keypress_immediate<F>(
        &self,
        lua: &mlua::Lua,
        event: mlua::Table,
        apply_layout: &mut F,
    ) -> Result<(), mlua::Error>
    where
        F: FnMut(&str) -> Result<(), String>,
    {
        self.events.emit_with_hook(lua, "keypress", event, || {
            if let Some(layout) = self.take_keyboard_layout_request() {
                apply_layout(&layout).map_err(mlua::Error::RuntimeError)?;
            }
            Ok(())
        })
    }

    pub fn set_keyboard_layout(&self, layout: String) {
        *self.pending_layout.lock().unwrap() = Some(layout);
    }

    pub fn take_keyboard_layout_request(&self) -> Option<String> {
        self.pending_layout.lock().unwrap().take()
    }
}
