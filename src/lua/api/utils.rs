use std::{
    fmt::Display,
    ops::{Div, Mul},
};

use mlua::{FromLua, IntoLua, MetaMethod, UserData};
use smithay::utils::{Coordinate, Logical, Point, Size};

use crate::lua_typedef;

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
                message: Some("expected a table for Size".to_string()),
            });
        };

        Ok(LuaSize(Size::new(
            table.get("width").or(table.get(0))?,
            table.get("height").or(table.get(1))?,
        )))
    }
}

impl<N, Kind> From<Size<N, Kind>> for LuaSize<N, Kind> {
    fn from(size: Size<N, Kind>) -> Self {
        LuaSize(size)
    }
}

pub struct LuaPoint<N, Kind>(pub Point<N, Kind>);

impl LuaPoint<f64, Logical> {
    pub fn create_ctor(lua: &mlua::Lua) -> mlua::Result<mlua::Function> {
        lua.create_function(|lua, args: mlua::MultiValue| {
            let mut args = args.into_iter();
            let first = args.next();
            let second = args.next();
            match (first, second) {
                (Some(mlua::Value::Number(x)), Some(mlua::Value::Number(y))) => {
                    Ok(LuaPoint(Point::new(x, y)))
                }
                (value, _) => Self::from_lua(value.unwrap_or(mlua::Nil), lua),
            }
        })
    }
}

trait Unit:
    'static + IntoLua + FromLua + Display + Coordinate + Mul<Output = Self> + Div<Output = Self>
{
}
impl<T> Unit for T where
    T: 'static + IntoLua + FromLua + Display + Coordinate + Mul<Output = Self> + Div<Output = Self>
{
}

impl<N: Unit, Kind: 'static> UserData for LuaPoint<N, Kind> {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("x", |_, this| Ok(this.0.x));
        fields.add_field_method_get("y", |_, this| Ok(this.0.y));
    }
    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        crate::lua_display_metamethods!(methods);

        methods.add_method_mut("add", |_, this, other: Self| {
            this.0 = this.0 + other.0;
            Ok(())
        });

        methods.add_meta_method(MetaMethod::Add, |_, _, (a, b): (Self, Self)| {
            Ok(Self(a.0 + b.0))
        });

        methods.add_meta_method(MetaMethod::Mul, |_, _, (a, b): (Self, Self)| {
            Ok(Self((a.0.x * b.0.x, a.0.y * b.0.y).into()))
        });
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
                message: Some("expected a table for Point".to_string()),
            });
        };

        Ok(LuaPoint(Point::new(
            table.get("x").or(table.get(0))?,
            table.get("y").or(table.get(1))?,
        )))
    }
}

impl<N, Kind> From<Point<N, Kind>> for LuaPoint<N, Kind> {
    fn from(point: Point<N, Kind>) -> Self {
        LuaPoint(point)
    }
}

lua_typedef!(Size => LuaSize<f64, Logical> {
    let width: number;
    let height: number;
});

lua_typedef!(Point => LuaPoint<f64, Logical> {
    fn(new: Point) -> Point;
    fn(x: number, y: number) -> Point;
    let x: number;
    let y: number;
    fn add(other: Point) -> nil;
});
