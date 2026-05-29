use mlua::{FromLua, IntoLua, UserData};
use smithay::utils::{Coordinate, Point, Size};

pub mod events;
pub mod wm;
pub mod wm_input;
pub mod wm_outputs;
pub mod wm_windows;

pub struct LuaSize<N, Kind>(Size<N, Kind>);

impl<N: IntoLua + Copy, Kind> UserData for LuaSize<N, Kind> {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("width", |_, this| Ok(this.0.w));
        fields.add_field_method_get("height", |_, this| Ok(this.0.h));
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

pub struct LuaPoint<N, Kind>(Point<N, Kind>);

impl<N: IntoLua + Copy, Kind> UserData for LuaPoint<N, Kind> {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("x", |_, this| Ok(this.0.x));
        fields.add_field_method_get("y", |_, this| Ok(this.0.y));
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
