use mlua::UserData;
use smithay::input::Seat;

use crate::{
    core::{GlobalHazel, Hazel},
    lua::api::{wm_input::LuaFocusTarget, wm_input_keyboard::WmInputKeyboard, wm_input_pointer::WmInputPointer},
    lua_typedef,
};

#[derive(Default)]
pub struct WmInputSeats;

impl UserData for WmInputSeats {
    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("count", |_, _, _: ()| {
            GlobalHazel::with(|hazel| hazel.compositor.seats.len())
        });

        methods.add_method("all", |_, _, _: ()| {
            GlobalHazel::with(|hazel| {
                let mut seats: Vec<_> = hazel.compositor.seats.values().cloned().map(WmSeat).collect();
                seats.sort_by_key(|seat| seat.0.name().to_owned());
                seats
            })
        });

        methods.add_method("get", |_, _, name: String| {
            GlobalHazel::with(|hazel| {
                hazel
                    .compositor
                    .seats
                    .get(&name)
                    .map(|seat| WmSeat(seat.clone()))
            })
        });
    }
}

lua_typedef!(WmSeats => WmInputSeats {
    fn count() -> integer;
    fn all() -> Array<Seat>;
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

        methods.add_method("focused", |_, this, _: ()| {
            GlobalHazel::try_with(|hazel| {
                Ok(hazel.compositor.keyboard_focus_target_of(&this.0).map(LuaFocusTarget))
            })
        });

        methods.add_method("focus", |_, this, target: Option<LuaFocusTarget>| {
            GlobalHazel::try_with(|hazel| {
                hazel.set_keyboard_focus(&this.0, target.as_ref().map(|t| &t.0));
                Ok(())
            })
        });
    }
}

lua_typedef!(Seat => WmSeat {
    let name: string;
    fn pointer() -> Option<Pointer>;
    fn keyboard() -> Option<Keyboard>;
    fn focused() -> Option<FocusTarget>;
    fn focus(target: Option<FocusTarget>) -> nil;
});
