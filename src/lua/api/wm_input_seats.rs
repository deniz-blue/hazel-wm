use mlua::UserData;
use smithay::input::Seat;

use crate::{
    core::{GlobalHazel, Hazel},
    lua::api::{wm_input_keyboard::WmInputKeyboard, wm_input_pointer::WmInputPointer},
    lua_typedef,
};

#[derive(Default)]
pub struct WmInputSeats;

impl UserData for WmInputSeats {
    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("count", |_, _, _: ()| {
            GlobalHazel::with(|hazel| Ok(hazel.compositor.seats.len()))
        });

        methods.add_method("get", |_, _, name: String| {
            GlobalHazel::with(|hazel| {
                Ok(hazel
                    .compositor
                    .seats
                    .get(&name)
                    .map(|seat| WmSeat(seat.clone())))
            })
        });
    }
}

lua_typedef!(WmSeats => WmInputSeats {
    fn count() -> integer;
    fn get(name: string) -> Option<Seat>;
});

pub struct WmSeat(pub Seat<Hazel>);

impl UserData for WmSeat {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("name", |_, this| Ok(this.0.name().to_string()));
    }

    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("pointer", |_, this, _: ()| {
            Ok(this.0.get_pointer().map(WmInputPointer))
        });

        methods.add_method("keyboard", |_, this, _: ()| {
            Ok(this.0.get_keyboard().map(WmInputKeyboard))
        });
    }
}

lua_typedef!(Seat => WmSeat {
    let name: string;
    fn pointer() -> Option<Pointer>;
    fn keyboard() -> Option<Keyboard>;
});
