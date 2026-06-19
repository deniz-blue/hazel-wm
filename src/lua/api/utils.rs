use std::{
    fmt::Display,
    ops::{Div, Mul},
};

use mlua::{FromLua, IntoLua, MetaMethod, UserData};
use smithay::utils::{Coordinate, Logical, Point, Rectangle, Size};

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
            this.0 += other.0;
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

pub struct LuaRect(pub Rectangle<i32, Logical>);

impl LuaRect {
    pub fn contains(&self, point: LuaPoint<i32, Logical>) -> bool {
        self.0.contains(point.0)
    }

    pub fn intersects(&self, other: &LuaRect) -> bool {
        self.0.overlaps(other.0)
    }

    pub fn intersection(&self, other: &LuaRect) -> Option<LuaRect> {
        self.0.intersection(other.0).map(LuaRect)
    }

    /// Returns the smallest rect that contains both.
    pub fn merge(&self, other: &LuaRect) -> LuaRect {
        use smithay::utils::Coordinate;
        let min_x = Coordinate::min(self.0.loc.x, other.0.loc.x);
        let min_y = Coordinate::min(self.0.loc.y, other.0.loc.y);
        let max_x = Coordinate::max(self.0.loc.x + self.0.size.w, other.0.loc.x + other.0.size.w);
        let max_y = Coordinate::max(self.0.loc.y + self.0.size.h, other.0.loc.y + other.0.size.h);
        LuaRect(Rectangle::from_extremities(
            (min_x, min_y),
            (max_x, max_y),
        ))
    }

    pub fn inset(&self, amount: i32) -> LuaRect {
        LuaRect(Rectangle::from_extremities(
            (self.0.loc.x + amount, self.0.loc.y + amount),
            (self.0.loc.x + self.0.size.w - amount, self.0.loc.y + self.0.size.h - amount),
        ))
    }
}

impl LuaRect {
    pub fn create_ctor(lua: &mlua::Lua) -> mlua::Result<mlua::Function> {
        lua.create_function(move |lua, args: mlua::MultiValue| {
            let table: mlua::Table = match args.len() {
                4 => lua.create_sequence_from(args.into_iter())?,
                1 => args.into_iter().next().and_then(|v| {
                    if let mlua::Value::Table(t) = v { Some(t) } else { None }
                }).ok_or_else(|| mlua::Error::FromLuaConversionError {
                    from: "value",
                    to: "Rect".to_owned(),
                    message: Some("expected a table or 4 numbers".to_string()),
                })?,
                _ => return Err(mlua::Error::FromLuaConversionError {
                    from: "arguments",
                    to: "Rect".to_owned(),
                    message: Some("expected 1 table or 4 numbers".to_string()),
                }),
            };
            let x: i32 = table.get("x").or(table.get(1))?;
            let y: i32 = table.get("y").or(table.get(2))?;
            let w: i32 = table.get("width").or(table.get(3))?;
            let h: i32 = table.get("height").or(table.get(4))?;
            Ok(LuaRect(Rectangle::new((x, y).into(), (w, h).into())))
        })
    }
}

impl FromLua for LuaRect {
    fn from_lua(value: mlua::Value, _: &mlua::Lua) -> mlua::Result<Self> {
        let mlua::Value::Table(table) = value else {
            return Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "Rect".to_owned(),
                message: Some("expected a table for Rect".to_string()),
            });
        };

        let x: i32 = table.get("x").or(table.get(1))?;
        let y: i32 = table.get("y").or(table.get(2))?;
        let w: i32 = table.get("width").or(table.get(3))?;
        let h: i32 = table.get("height").or(table.get(4))?;

        Ok(LuaRect(Rectangle::new((x, y).into(), (w, h).into())))
    }
}

impl Display for LuaRect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Rect {{ x: {}, y: {}, width: {}, height: {} }}",
            self.0.loc.x, self.0.loc.y, self.0.size.w, self.0.size.h
        )
    }
}

impl UserData for LuaRect {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("x", |_, this| Ok(this.0.loc.x));
        fields.add_field_method_get("y", |_, this| Ok(this.0.loc.y));
        fields.add_field_method_get("width", |_, this| Ok(this.0.size.w));
        fields.add_field_method_get("height", |_, this| Ok(this.0.size.h));
    }

    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        crate::lua_display_metamethods!(methods);

        methods.add_method("contains", |_, this, point: LuaPoint<i32, _>| {
            Ok(this.contains(point))
        });
        methods.add_method("intersects", |_, this, other: LuaRect| {
            Ok(this.intersects(&other))
        });
        methods.add_method("intersection", |_, this, other: LuaRect| {
            Ok(this.intersection(&other))
        });
        methods.add_method("merge", |_, this, other: LuaRect| {
            Ok(this.merge(&other))
        });
        methods.add_method("inset", |_, this, amount: i32| {
            Ok(this.inset(amount))
        });
    }
}

lua_typedef!(Rect => LuaRect {
    fn(x: number, y: number, width: number, height: number) -> Rect;
    fn(table: table) -> Rect;
    let x: number;
    let y: number;
    let width: number;
    let height: number;
    fn contains(point: Point) -> boolean;
    fn intersects(other: Rect) -> boolean;
    fn intersection(other: Rect) -> Rect;
    fn merge(other: Rect) -> Rect;
    fn inset(amount: number) -> Rect;
});

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
