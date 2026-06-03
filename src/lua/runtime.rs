use std::{path::Path, process::Stdio, rc::Rc};

use calloop_notify::notify::{RecursiveMode, Watcher};
use mlua::{IntoLua, Lua, Result as LuaResult};

use crate::{
    core::{GlobalHazel, HazelEventLoop},
    lua::api::{
        utils::LuaPoint,
        wm::Wm,
        wm_input_sym::{LuaKeys, LuaMouseButtons},
    },
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

    pub fn init(&mut self) -> LuaResult<()> {
        self.lua = Lua::new();
        self.wm = Default::default();

        self.init_globals()?;

        self.lua
            .load(r#"package.path = package.path .. ";./test/?.lua""#)
            .exec()?;

        self.lua.load(r#"require("main")"#).exec()?;

        Ok(())
    }

    pub fn init_globals(&self) -> LuaResult<()> {
        let globals = self.lua.globals();

        globals.set("wm", self.wm.clone())?;

        globals.set("Key", LuaKeys)?;
        globals.set("Button", LuaMouseButtons)?;

        globals.set("Point", LuaPoint::<f64, _>::create_ctor(&self.lua)?)?;

        globals.set(
            "spawn",
            self.lua
                .create_function(|_, (cmd, args): (String, Option<Vec<String>>)| {
                    std::process::Command::new(cmd)
                        .args(args.unwrap_or_default())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .stdin(Stdio::null())
                        .spawn()
                        .map_err(|e| mlua::Error::external(e))?;
                    Ok(())
                })?,
        )?;

        globals.set(
            "exec",
            self.lua
                .create_function(|lua, (cmd, args): (String, Option<Vec<String>>)| {
                    let output = std::process::Command::new(cmd)
                        .args(args.unwrap_or_default())
                        .output()
                        .map_err(|e| mlua::Error::external(e))?;
                    Ok(lua.create_table_from([
                        (
                            "stdout",
                            output
                                .stdout
                                .into_iter()
                                .map(|b| b as char)
                                .collect::<String>()
                                .into_lua(lua)?,
                        ),
                        (
                            "stderr",
                            output
                                .stderr
                                .into_iter()
                                .map(|b| b as char)
                                .collect::<String>()
                                .into_lua(lua)?,
                        ),
                        ("status", output.status.code().into_lua(lua)?),
                    ]))
                })?,
        )?;

        Ok(())
    }

    pub fn listen(
        &self,
        event_loop: &mut HazelEventLoop,
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut notify_source = calloop_notify::NotifySource::new()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        notify_source
            .watch(Path::new("./test"), RecursiveMode::Recursive)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
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
            })
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

        Ok(())
    }
}
