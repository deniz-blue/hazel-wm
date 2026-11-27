use std::{
    cmp::{max, min},
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

impl<N: FromLua + Coordinate + 'static, Kind: 'static> FromLua for LuaSize<N, Kind> {
    fn from_lua(value: mlua::Value, _: &mlua::Lua) -> mlua::Result<Self> {
        if let mlua::Value::UserData(ud) = &value {
            if let Ok(size) = ud.borrow::<LuaSize<N, Kind>>() {
                return Ok(LuaSize(size.0));
            }
            if let Ok(size) = ud.borrow::<LuaSize<f64, Kind>>() {
                return Ok(LuaSize(Size::new(N::from_f64(size.0.w), N::from_f64(size.0.h))));
            }
            if let Ok(size) = ud.borrow::<LuaSize<i32, Kind>>() {
                return Ok(LuaSize(Size::new(
                    N::from_f64(size.0.w.to_f64()),
                    N::from_f64(size.0.h.to_f64()),
                )));
            }
        }
        let mlua::Value::Table(table) = value else {
            return Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "Size".to_owned(),
                message: Some("expected a table for Size".to_string()),
            });
        };

        let width: N = table.get("width").or(table.get(1))?;
        let height: N = table.get("height").or(table.get(2))?;
        if !(width.non_negative() && height.non_negative()) {
            return Err(mlua::Error::external("a size cannot be negative or NaN"));
        }
        Ok(LuaSize(Size::new(width, height)))
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
                (Some(mlua::Value::Integer(x)), Some(mlua::Value::Integer(y))) => {
                    Ok(LuaPoint(Point::new(x as f64, y as f64)))
                }
                (Some(mlua::Value::Number(x)), Some(mlua::Value::Integer(y))) => {
                    Ok(LuaPoint(Point::new(x, y as f64)))
                }
                (Some(mlua::Value::Integer(x)), Some(mlua::Value::Number(y))) => {
                    Ok(LuaPoint(Point::new(x as f64, y)))
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

impl<N: FromLua + Coordinate + 'static, Kind: 'static> FromLua for LuaPoint<N, Kind> {
    fn from_lua(value: mlua::Value, _: &mlua::Lua) -> mlua::Result<Self> {
        if let mlua::Value::UserData(ud) = &value {
            if let Ok(point) = ud.borrow::<LuaPoint<N, Kind>>() {
                return Ok(LuaPoint(point.0));
            }
            if let Ok(point) = ud.borrow::<LuaPoint<f64, Kind>>() {
                return Ok(LuaPoint(Point::new(N::from_f64(point.0.x), N::from_f64(point.0.y))));
            }
            if let Ok(point) = ud.borrow::<LuaPoint<i32, Kind>>() {
                return Ok(LuaPoint(Point::new(
                    N::from_f64(point.0.x.to_f64()),
                    N::from_f64(point.0.y.to_f64()),
                )));
            }
        }
        let mlua::Value::Table(table) = value else {
            return Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "Point".to_owned(),
                message: Some("expected a table for Point".to_string()),
            });
        };

        Ok(LuaPoint(Point::new(
            table.get("x").or(table.get(1))?,
            table.get("y").or(table.get(2))?,
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

    pub fn merge(&self, other: &LuaRect) -> mlua::Result<LuaRect> {
        let (a, b) = (&self.0, &other.0);
        let min_x = i64::from(min(a.loc.x, b.loc.x));
        let min_y = i64::from(min(a.loc.y, b.loc.y));
        let max_x = max(i64::from(a.loc.x) + i64::from(a.size.w), i64::from(b.loc.x) + i64::from(b.size.w));
        let max_y = max(i64::from(a.loc.y) + i64::from(a.size.h), i64::from(b.loc.y) + i64::from(b.size.h));
        Self::from_extents(min_x, min_y, max_x - min_x, max_y - min_y)
    }

    pub fn inset(&self, amount: i32) -> mlua::Result<LuaRect> {
        let amount = i64::from(amount);
        let rect = &self.0;
        Self::from_extents(
            i64::from(rect.loc.x) + amount,
            i64::from(rect.loc.y) + amount,
            max(i64::from(rect.size.w) - 2 * amount, 0),
            max(i64::from(rect.size.h) - 2 * amount, 0),
        )
    }

    pub fn from_extents(x: i64, y: i64, width: i64, height: i64) -> mlua::Result<LuaRect> {
        let fits = |value: i64| i32::try_from(value).is_ok();
        if width < 0 || height < 0 {
            return Err(mlua::Error::external("a rect cannot have a negative size"));
        }
        if !(fits(x) && fits(y) && fits(width) && fits(height) && fits(x + width) && fits(y + height)) {
            return Err(mlua::Error::external("a rect must stay within the 32-bit coordinate range"));
        }
        Ok(LuaRect(Rectangle::new((x as i32, y as i32).into(), (width as i32, height as i32).into())))
    }

    pub fn from_table(table: &mlua::Table) -> mlua::Result<LuaRect> {
        let x: i32 = table.get("x").or(table.get(1))?;
        let y: i32 = table.get("y").or(table.get(2))?;
        let width: i32 = table.get("width").or(table.get(3))?;
        let height: i32 = table.get("height").or(table.get(4))?;
        Self::from_extents(x.into(), y.into(), width.into(), height.into())
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
            LuaRect::from_table(&table)
        })
    }
}

impl FromLua for LuaRect {
    fn from_lua(value: mlua::Value, _: &mlua::Lua) -> mlua::Result<Self> {
        if let mlua::Value::UserData(ud) = &value {
            if let Ok(rect) = ud.borrow::<LuaRect>() {
                return Ok(LuaRect(rect.0));
            }
        }
        let mlua::Value::Table(table) = value else {
            return Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "Rect".to_owned(),
                message: Some("expected a table for Rect".to_string()),
            });
        };

        LuaRect::from_table(&table)
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
        methods.add_method("merge", |_, this, other: LuaRect| this.merge(&other));
        methods.add_method("inset", |_, this, amount: i32| this.inset(amount));
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
    fn intersection(other: Rect) -> Option<Rect>;
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
