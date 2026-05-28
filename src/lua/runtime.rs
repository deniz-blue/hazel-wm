use mlua::Lua;

pub struct HazelLua {
    lua: Lua,
}

impl Default for HazelLua {
    fn default() -> Self {
        Self { lua: Lua::new() }
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

        self.lua
            .load(r#"package.path = package.path .. ";./test/?.lua""#)
            .exec()?;

        self.lua.load(r#"require("main")"#).exec()?;

        Ok(())
    }
}
