use mlua::Lua;
use smithay::utils::{Logical, Point, Size};

use crate::lua::api::utils::{LuaPoint, LuaSize};

#[test]
fn integer_points_are_accepted_where_float_points_are_expected() {
    let lua = Lua::new();
    let sum = lua
        .create_function(|_, point: LuaPoint<f64, Logical>| Ok(point.0.x + point.0.y))
        .unwrap();
    let point = lua
        .create_userdata(LuaPoint::<i32, Logical>(Point::new(3, 4)))
        .unwrap();
    assert_eq!(sum.call::<f64>(point).unwrap(), 7.0);
}

#[test]
fn integer_sizes_are_accepted_where_float_sizes_are_expected() {
    let lua = Lua::new();
    let area = lua
        .create_function(|_, size: LuaSize<f64, Logical>| Ok(size.0.w * size.0.h))
        .unwrap();
    let size = lua
        .create_userdata(LuaSize::<i32, Logical>(Size::new(3, 4)))
        .unwrap();
    assert_eq!(area.call::<f64>(size).unwrap(), 12.0);
}
