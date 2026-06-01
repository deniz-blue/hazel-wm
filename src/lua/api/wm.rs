use std::rc::Rc;

use mlua::UserData;

use crate::{
    core::GlobalHazel,
    impl_lua_event_handler, impl_lua_event_source,
    lua::{
        api::{wm_input::WmInput, wm_outputs::WmOutputs, wm_windows::WmWindows},
        event_handler::LuaEventHandler,
    },
    lua_typedef,
};

#[derive(Default)]
pub struct Wm {
    pub events: LuaEventHandler,
    pub input: Rc<WmInput>,
    pub windows: Rc<WmWindows>,
    pub outputs: Rc<WmOutputs>,
}

impl_lua_event_source!(Wm);

impl UserData for Wm {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("name", |_, _| Ok("Hazel"));
        fields.add_field_method_get("input", |_, this| Ok(this.input.clone()));
        fields.add_field_method_get("windows", |_, this| Ok(this.windows.clone()));
        fields.add_field_method_get("outputs", |_, this| Ok(this.outputs.clone()));
    }

    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        impl_lua_event_handler!(methods);

        methods.add_method("quit", |_, _this, ()| {
            GlobalHazel::with(|hazel| Ok(hazel.loop_signal.stop()))
        });
    }
}

lua_typedef!(Wm => Wm {
    extern wm;
    let name: string;
    let input: WmInput;
    let windows: WmWindows;
    let outputs: WmOutputs;
	use ready => nil;
    fn quit() -> nil;
});
