use std::fmt::Display;

use mlua::{FromLua, IntoLua, UserData};
use smithay::utils::{Coordinate, Point, Size};

pub struct LuaSize<N, Kind>(pub Size<N, Kind>);

impl<N: IntoLua + Copy + Display, Kind> UserData for LuaSize<N, Kind> {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("width", |_, this| Ok(this.0.w));
        fields.add_field_method_get("height", |_, this| Ok(this.0.h));
    }
    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        crate::lua_display_metamethods!(methods);
    }
}

impl<N: Display, Kind> Display for LuaSize<N, Kind> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Size {{ width: {}, height: {} }}", self.0.w, self.0.h)
    }
}

impl<N: FromLua + Coordinate, Kind> FromLua for LuaSize<N, Kind> {
    fn from_lua(value: mlua::Value, _: &mlua::Lua) -> mlua::Result<Self> {
        let mlua::Value::Table(table) = value else {
            return Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "Size".to_owned(),
                message: Some("expected a table with 'width' and 'height' fields".to_string()),
            });
        };

        Ok(LuaSize(Size::new(
            table.get("width")?,
            table.get("height")?,
        )))
    }
}

impl<N, Kind> From<Size<N, Kind>> for LuaSize<N, Kind> {
    fn from(size: Size<N, Kind>) -> Self {
        LuaSize(size)
    }
}

pub struct LuaPoint<N, Kind>(pub Point<N, Kind>);

impl<N: IntoLua + Copy + Display, Kind> UserData for LuaPoint<N, Kind> {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("x", |_, this| Ok(this.0.x));
        fields.add_field_method_get("y", |_, this| Ok(this.0.y));
    }
    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        crate::lua_display_metamethods!(methods);
    }
}

impl<N: Display, Kind> Display for LuaPoint<N, Kind> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Point {{ x: {}, y: {} }}", self.0.x, self.0.y)
    }
}

impl<N: FromLua + Coordinate, Kind> FromLua for LuaPoint<N, Kind> {
    fn from_lua(value: mlua::Value, _: &mlua::Lua) -> mlua::Result<Self> {
        let mlua::Value::Table(table) = value else {
            return Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "Point".to_owned(),
                message: Some("expected a table with 'x' and 'y' fields".to_string()),
            });
        };

        Ok(LuaPoint(Point::new(table.get("x")?, table.get("y")?)))
    }
}

impl<N, Kind> From<Point<N, Kind>> for LuaPoint<N, Kind> {
    fn from(point: Point<N, Kind>) -> Self {
        LuaPoint(point)
    }
}