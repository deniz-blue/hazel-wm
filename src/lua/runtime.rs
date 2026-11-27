use std::{path::PathBuf, rc::Rc};

use calloop_notify::notify::{RecursiveMode, Watcher};
use mlua::{Lua, Result as LuaResult, Table};
use tracing::info;

use crate::{
    core::{GlobalHazel, HazelEventLoop},
    lua::api::{
        process,
        utils::{LuaPoint, LuaRect},
        wm::{STORE_REGISTRY_KEY, Wm},
        wm_input_sym::{LuaKeys, LuaMouseButtons},
    },
    lua::store::{StoreValue, snapshot},
};

pub fn compositor_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("HAZEL_COMPOSITOR_DIR") {
        return PathBuf::from(dir);
    }
    let user_dir = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .map(|data| data.join("hazel/compositor"));
    match user_dir {
        Some(dir) if dir.join("main.lua").is_file() => dir,
        _ => PathBuf::from("./test"),
    }
}

pub struct HazelLua {
    pub lua: Lua,
    pub wm: Rc<Wm>,
    pub loaded: bool,
}

impl HazelLua {
    pub fn new_uninit() -> Self {
        Self {
            lua: Lua::new(),
            wm: Default::default(),
            loaded: false,
        }
    }

    pub fn load(store: Option<Vec<(StoreValue, StoreValue)>>, is_reload: bool) -> LuaResult<Self> {
        let this = Self {
            lua: Lua::new(),
            wm: Rc::new(Wm::new(is_reload)),
            loaded: true,
        };
        this.init_globals()?;

        let store = StoreValue::table_from(store.unwrap_or_default(), &this.lua)?;
        this.lua.set_named_registry_value(STORE_REGISTRY_KEY, store)?;

        let dir = compositor_dir();
        this.lua
            .load(format!(
                r#"package.path = package.path .. ";{}/?.lua""#,
                dir.display()
            ))
            .exec()?;
        this.lua.load(r#"require("main")"#).exec()?;
        Ok(this)
    }

    pub fn snapshot_store(&self) -> LuaResult<Option<Vec<(StoreValue, StoreValue)>>> {
        match self.lua.named_registry_value::<Table>(STORE_REGISTRY_KEY) {
            Ok(table) => snapshot(&table).map(Some),
            Err(_) => Ok(None),
        }
    }

    pub fn init_globals(&self) -> LuaResult<()> {
        let globals = self.lua.globals();

        globals.set("wm", self.wm.clone())?;

        globals.set("Key", LuaKeys)?;
        globals.set("Button", LuaMouseButtons)?;

        globals.set("Point", LuaPoint::<f64, _>::create_ctor(&self.lua)?)?;
        globals.set("Rect", LuaRect::create_ctor(&self.lua)?)?;

        process::register(&self.lua, self.wm.processes.clone())?;

        let tostring: mlua::Function = globals.get("tostring")?;
        globals.set(
            "print",
            self.lua
                .create_function(move |_, args: mlua::Variadic<mlua::Value>| {
                    let output = args
                        .into_iter()
                        .map(|v| tostring.call::<String>(v))
                        .collect::<Result<Vec<_>, _>>()?
                        .join("\t");
                    info!("print(): {output}");
                    Ok(())
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
            .watch(&compositor_dir(), RecursiveMode::Recursive)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        event_loop
            .handle()
            .insert_source(notify_source, move |event, _, state| {
                if !event.kind.is_modify() {
                    return;
                }

                GlobalHazel::execute(state, |hazel| hazel.load_lua());
            })
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

        Ok(())
    }
}
