use std::cell::RefCell;

use mlua::UserData;
use smithay::{
    input::pointer::{ButtonEvent, MotionEvent, PointerHandle},
    utils::{Logical, Point},
};

use crate::{
    core::{GlobalHazel, Hazel},
    lua::api::utils::LuaPoint,
};

pub struct WmInputPointer(pub PointerHandle<Hazel>);

impl WmInputPointer {
    pub fn position(&self) -> Result<LuaPoint<f64, Logical>, mlua::Error> {
        Ok(LuaPoint(self.0.current_location()))
    }

    pub fn buttons(&self) -> Result<Vec<u32>, mlua::Error> {
        GlobalHazel::with(|hazel| Ok(hazel.compositor.pointer_pressed.get(&self.0).cloned().unwrap_or_default()))
    }
}

impl UserData for WmInputPointer {
    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("position", |_, this, _: ()| this.position());
        methods.add_method("buttons", |_, this, _: ()| this.buttons());
    }
}

pub struct LuaPointerButtonEvent {
    pub event: ButtonEvent,
    pub pointer: PointerHandle<Hazel>,
    pub default_prevented: RefCell<bool>,
}

impl LuaPointerButtonEvent {
    pub fn name() -> String {
        String::from("pointer_button")
    }
}

impl UserData for LuaPointerButtonEvent {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("button", |_, this| Ok(this.event.button));
        fields.add_field_method_get("state", |_, this| Ok(format!("{:?}", this.event.state)));
        fields.add_field_method_get("pointer", |_, this| {
            Ok(WmInputPointer(this.pointer.clone()))
        });
    }

    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("prevent_default", |_, this, _: ()| {
            this.default_prevented.replace(true);
            Ok(())
        });
    }
}

#[derive(Clone)]
pub struct LuaPointerMotionEvent {
    pub event: MotionEvent,
    pub pointer: PointerHandle<Hazel>,
    pub output_position: Option<Point<f64, Logical>>,
    pub default_prevented: RefCell<bool>,
}

impl LuaPointerMotionEvent {
    pub fn name() -> String {
        String::from("pointer_move")
    }
}

impl UserData for LuaPointerMotionEvent {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("position", |_, this| Ok(LuaPoint(this.event.location)));
        fields.add_field_method_get("pointer", |_, this| {
            Ok(WmInputPointer(this.pointer.clone()))
        });
        fields.add_field_method_get("output_position", |_, this| {
            Ok(this.output_position.map(LuaPoint))
        });
    }

    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("prevent_default", |_, this, _: ()| {
            this.default_prevented.replace(true);
            Ok(())
        });
    }
}
