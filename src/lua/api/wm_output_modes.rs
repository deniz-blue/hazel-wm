use mlua::{FromLua, IntoLua, Lua, Table, Value};
use smithay::{
    output::Mode,
    utils::{Physical, Transform},
};

use crate::{lua::api::utils::LuaSize, lua_typedef};

const TRANSFORMS: [(&str, Transform); 8] = [
    ("normal", Transform::Normal),
    ("90", Transform::_90),
    ("180", Transform::_180),
    ("270", Transform::_270),
    ("flipped", Transform::Flipped),
    ("flipped_90", Transform::Flipped90),
    ("flipped_180", Transform::Flipped180),
    ("flipped_270", Transform::Flipped270),
];

pub fn transform_name(transform: Transform) -> &'static str {
    TRANSFORMS
        .iter()
        .find(|(_, candidate)| *candidate == transform)
        .map_or("normal", |(name, _)| name)
}

pub fn parse_transform(name: &str) -> mlua::Result<Transform> {
    TRANSFORMS
        .iter()
        .find(|(candidate, _)| *candidate == name)
        .map(|(_, transform)| *transform)
        .ok_or_else(|| {
            let names: Vec<_> = TRANSFORMS.iter().map(|(name, _)| *name).collect();
            mlua::Error::external(format!("unknown transform {name:?}, expected one of {}", names.join(", ")))
        })
}

pub struct LuaOutputMode {
    pub mode: Mode,
    pub is_current: bool,
    pub is_preferred: bool,
}

impl IntoLua for LuaOutputMode {
    fn into_lua(self, lua: &Lua) -> mlua::Result<Value> {
        let table = lua.create_table()?;
        table.set("size", LuaSize(self.mode.size).into_lua(lua)?)?;
        table.set("refresh", self.mode.refresh)?;
        table.set("current", self.is_current)?;
        table.set("preferred", self.is_preferred)?;
        Ok(Value::Table(table))
    }
}

impl FromLua for LuaOutputMode {
    fn from_lua(value: Value, lua: &Lua) -> mlua::Result<Self> {
        let Value::Table(table) = value else {
            return Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "OutputMode".to_owned(),
                message: Some("expected a table with size and refresh".to_owned()),
            });
        };
        let size = LuaSize::<i32, Physical>::from_lua(table.get::<Value>("size")?, lua)?;
        let refresh = table.get::<Option<i32>>("refresh")?.unwrap_or(0);
        Ok(Self { mode: Mode { size: size.0, refresh }, is_current: false, is_preferred: false })
    }
}

pub fn mode_list(lua: &Lua, modes: Vec<LuaOutputMode>) -> mlua::Result<Table> {
    lua.create_sequence_from(modes)
}

lua_typedef!(OutputMode => LuaOutputMode {
    let size: Size;
    let refresh: number;
    let current: boolean;
    let preferred: boolean;
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_transform_name_round_trips() {
        for (name, transform) in TRANSFORMS {
            assert_eq!(parse_transform(name).unwrap(), transform);
            assert_eq!(transform_name(transform), name);
        }
    }

    #[test]
    fn an_unknown_transform_lists_the_valid_names() {
        let message = parse_transform("sideways").unwrap_err().to_string();
        assert!(message.contains("flipped_270"), "{message}");
    }

    #[test]
    fn a_mode_table_without_refresh_asks_for_any_refresh() {
        let lua = Lua::new();
        let mode: LuaOutputMode = lua.load("return { size = { width = 800, height = 600 } }").eval().unwrap();
        assert_eq!((mode.mode.size.w, mode.mode.size.h, mode.mode.refresh), (800, 600, 0));
    }
}
