use mlua::UserData;

use crate::{
    impl_lua_event_handler, impl_lua_event_source,
    lua::{api::wm_input_seats::WmInputSeats, event_handler::LuaEventHandler},
    lua_typedef,
};

#[derive(Default)]
pub struct WmInput {
    pub events: LuaEventHandler,
}

impl_lua_event_source!(WmInput);

impl UserData for WmInput {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("seats", |_, _| Ok(WmInputSeats));
    }

    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        impl_lua_event_handler!(methods);
    }
}

lua_typedef!(WmInput => WmInput {
    let seats: WmSeats;
    use key => KeyEvent;
    use pointer_move => PointerMoveEvent;
    use pointer_button => PointerButtonEvent;
    use new_keyboard => Keyboard;
    use new_pointer => Pointer;
});
