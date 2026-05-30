use std::cell::RefCell;

use mlua::UserData;
use smithay::{
    input::pointer::{ButtonEvent, MotionEvent},
    utils::{Logical, Point},
};

use crate::{core::GlobalHazel, lua::api::utils::LuaPoint};

pub struct WmInputPointer;

impl WmInputPointer {
    pub fn position() -> Result<LuaPoint<f64, Logical>, mlua::Error> {
        GlobalHazel::with(|hazel| {
            Ok(hazel
                .compositor
                .seat
                .get_pointer()
                .map(|pointer| LuaPoint(pointer.current_location()))
                .unwrap_or_else(|| LuaPoint(Point::new(0.0, 0.0))))
        })
    }

    pub fn buttons() -> Result<Vec<u32>, mlua::Error> {
        GlobalHazel::with(|hazel| Ok(hazel.pointer_pressed.clone()))
    }
}

impl UserData for WmInputPointer {
    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_function("position", |_, ()| Self::position());
        methods.add_function("buttons", |_, ()| Self::buttons());
    }
}

pub struct LuaPointerButtonEvent(pub ButtonEvent);

impl LuaPointerButtonEvent {
    pub fn name() -> String {
        String::from("pointer_button")
    }
}

impl UserData for LuaPointerButtonEvent {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("button", |_, this| Ok(this.0.button));
        fields.add_field_method_get("state", |_, this| Ok(format!("{:?}", this.0.state)));
    }
}

#[derive(Clone)]
pub struct LuaPointerMotionEvent {
	pub event: MotionEvent,
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
    }

	fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
		methods.add_method("prevent_default", |_, this, _: ()| {
			this.default_prevented.replace(true);
			Ok(())
		});
	}
}
