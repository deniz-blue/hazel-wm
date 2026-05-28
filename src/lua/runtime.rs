use std::sync::Arc;

use mlua::Lua;

use crate::lua::api::wm::Wm;

pub struct HazelLua {
    pub lua: Lua,
    pub wm: Arc<Wm>,
}

impl Default for HazelLua {
    fn default() -> Self {
        Self {
            lua: Lua::new(),
            wm: Default::default(),
        }
    }
}

impl HazelLua {
    pub fn new() -> Self {
        let mut this = Self::default();
        if let Err(err) = this.reload() {
            eprintln!("Failed to initialize Lua runtime: {err}");
        }
        this
    }

    pub fn reload(&mut self) -> Result<(), mlua::Error> {
        self.lua = Lua::new();
        self.wm = Default::default();

        let globals = self.lua.globals();
        globals.set("wm", self.wm.clone())?;

        self.lua
            .load(r#"package.path = package.path .. ";./test/?.lua""#)
            .exec()?;

        self.lua.load(r#"require("main")"#).exec()?;

        Ok(())
    }
}
