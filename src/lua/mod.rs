use mlua::Lua;

use crate::lua::event_handler::LuaEventHandler;

pub mod event_handler;

pub struct HazelLua {
    pub lua: Lua,
	pub input_events: LuaEventHandler,
}

impl HazelLua {
    pub fn new() -> Self {
        let lua = Lua::new();

        lua.load(r#"package.path = package.path .. ";./test/?.lua""#)
            .exec()
            .expect("Failed to set Lua package path");

        let mut this = Self {
			lua,
			input_events: LuaEventHandler::new(),
		};

        if let Err(e) = this.init() {
            eprintln!("Failed to initialize Lua: {}", e);
        }

        this
    }

    pub fn init(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let globals = self.lua.globals();
		
        globals.set("input", self.input_events.clone())?;

        self.lua.load(r#"require("main")"#).exec()?;

        Ok(())
    }
}
