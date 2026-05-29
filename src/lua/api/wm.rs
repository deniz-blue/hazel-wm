use std::rc::Rc;

use mlua::UserData;

use crate::{
    core::GlobalHazel,
    impl_lua_event_handler, impl_lua_event_source,
    lua::{api::wm_input::WmInput, event_handler::LuaEventHandler},
};

#[derive(Default)]
pub struct Wm {
    pub events: LuaEventHandler,
    pub input: Rc<WmInput>,
}

impl_lua_event_source!(Wm);

impl UserData for Wm {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("name", |_, _| Ok("Hazel"));
        fields.add_field_method_get("input", |_, this| Ok(this.input.clone()));
    }

    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        impl_lua_event_handler!(methods);

        methods.add_method("quit", |_, _this, ()| {
            GlobalHazel::with(|hazel| Ok(hazel.loop_signal.stop()))
        });
    }
}
