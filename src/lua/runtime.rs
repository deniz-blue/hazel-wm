use std::{path::Path, rc::Rc};

use calloop_notify::notify::{RecursiveMode, Watcher};
use mlua::Lua;

use crate::{
    core::{GlobalHazel, HazelEventLoop},
    lua::api::wm::Wm,
};

pub struct HazelLua {
    pub lua: Lua,
    pub wm: Rc<Wm>,
}

impl HazelLua {
    pub fn new_uninit() -> Self {
        Self {
            lua: Lua::new(),
            wm: Default::default(),
        }
    }

    pub fn init(&mut self) -> Result<(), mlua::Error> {
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

    pub fn listen(
        &self,
        event_loop: &mut HazelEventLoop,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut notify_source = calloop_notify::NotifySource::new()?;
        notify_source.watch(Path::new("./test"), RecursiveMode::Recursive)?;
        event_loop
            .handle()
            .insert_source(notify_source, move |event, _, state| {
                if !event.kind.is_modify() {
                    return;
                }

                GlobalHazel::execute(state, |hazel| {
                    if let Err(e) = hazel.lua.init() {
                        eprintln!("Error reloading Lua: {e}");
                    } else {
                        println!("Reloaded Lua");
                    }
                });
            })?;

        Ok(())
    }
}
