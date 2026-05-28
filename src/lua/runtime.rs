use std::sync::Arc;

use mlua::Lua;

use crate::lua::{HazelHandle, api::wm::Wm};

pub struct HazelLua {
    pub lua: Lua,
    pub wm: Option<Arc<Wm>>,
}

impl HazelLua {
    pub fn new_uninit() -> Self {
        Self {
            lua: Lua::new(),
            wm: None,
        }
    }

    pub fn wm(&self) -> Arc<Wm> {
        self.wm.as_ref().unwrap().clone()
    }

    pub fn init(&mut self, hazel: HazelHandle) -> Result<(), mlua::Error> {
        self.lua = Lua::new();
        self.wm = Some(Arc::new(Wm::new(hazel)));

        let globals = self.lua.globals();
        globals.set("wm", self.wm.clone())?;

        self.lua
            .load(r#"package.path = package.path .. ";./test/?.lua""#)
            .exec()?;

        self.lua.load(r#"require("main")"#).exec()?;

        Ok(())
    }
}
