use mlua::UserData;
use smithay::input::pointer::PointerHandle;

use crate::{
    core::Hazel,
    lua::api::{wm_input_pointer::WmInputPointer, wm_windows::WmWindow},
    lua_typedef,
};

#[derive(Clone)]
pub struct WindowDragRequest {
    pub window: WmWindow,
    pub pointer: PointerHandle<Hazel>,
    pub edges: Option<&'static str>,
}

impl UserData for WindowDragRequest {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("window", |_, this| Ok(this.window.clone()));
        fields.add_field_method_get("pointer", |_, this| Ok(WmInputPointer(this.pointer.clone())));
        fields.add_field_method_get("edges", |_, this| Ok(this.edges));
    }
}

lua_typedef!(WindowDragRequest => WindowDragRequest {
    let window: Window;
    let pointer: Pointer;
    let edges: Option<string>;
});
