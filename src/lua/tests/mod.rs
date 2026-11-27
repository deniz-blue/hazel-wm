use mlua::Lua;

use crate::lua::api::utils::{LuaPoint, LuaRect};

mod floating;
mod focus;
mod keys;
mod layers;
mod odd_arguments;
mod outputs;
mod pointer;
mod points;
mod requests;
mod rules;
mod session;
mod bar;
mod titlebars;
mod tiling;

const MOCK_ENVIRONMENT: &str = include_str!("mock.lua");

const MODULES: [(&str, &str); 16] = [
    ("keys", include_str!("../../../test/keys.lua")),
    ("tiling", include_str!("../../../test/tiling.lua")),
    ("floating", include_str!("../../../test/floating.lua")),
    ("requests", include_str!("../../../test/requests.lua")),
    ("seats", include_str!("../../../test/seats.lua")),
    ("focus", include_str!("../../../test/focus.lua")),
    ("pointer", include_str!("../../../test/pointer.lua")),
    ("borders", include_str!("../../../test/borders.lua")),
    ("rules", include_str!("../../../test/rules.lua")),
    ("autostart", include_str!("../../../test/autostart.lua")),
    ("session", include_str!("../../../test/session.lua")),
    ("lock", include_str!("../../../test/lock.lua")),
    ("outputs", include_str!("../../../test/outputs.lua")),
    ("titlebars", include_str!("../../../test/titlebars.lua")),
    ("bar", include_str!("../../../test/bar.lua")),
    ("main", include_str!("../../../test/main.lua")),
];

pub fn lua_with_modules() -> Lua {
    let lua = Lua::new();
    let globals = lua.globals();
    globals
        .set("Point", LuaPoint::<f64, _>::create_ctor(&lua).unwrap())
        .unwrap();
    globals
        .set("Rect", LuaRect::create_ctor(&lua).unwrap())
        .unwrap();
    lua.load(MOCK_ENVIRONMENT).exec().unwrap();

    let preload: mlua::Table = globals
        .get::<mlua::Table>("package")
        .unwrap()
        .get("preload")
        .unwrap();
    for (name, source) in MODULES {
        let loader = lua
            .create_function(move |lua, _: mlua::MultiValue| lua.load(source).eval::<mlua::Value>())
            .unwrap();
        preload.set(name, loader).unwrap();
    }
    drop(globals);
    lua
}

pub fn eval<T: mlua::FromLua>(lua: &Lua, code: &str) -> T {
    lua.load(code).eval().unwrap()
}
