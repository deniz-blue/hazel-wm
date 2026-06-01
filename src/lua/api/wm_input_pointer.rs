use std::cell::RefCell;

use mlua::UserData;
use smithay::{
    backend::input::ButtonState, input::pointer::{ButtonEvent, MotionEvent, PointerHandle, RelativeMotionEvent}, utils::{Logical, Point, Serial}
};

use crate::{
    core::{GlobalHazel, Hazel},
    lua::api::{utils::LuaPoint, wm_input_sym::LuaMouseButton},
    lua_typedef,
};

pub struct WmInputPointer(pub PointerHandle<Hazel>);

impl WmInputPointer {
    pub fn position(&self) -> Result<LuaPoint<f64, Logical>, mlua::Error> {
        Ok(LuaPoint(self.0.current_location()))
    }

    pub fn buttons(&self) -> Result<Vec<u32>, mlua::Error> {
        GlobalHazel::with(|hazel| {
            Ok(hazel
                .compositor
                .pointer_pressed
                .get(&self.0)
                .cloned()
                .unwrap_or_default())
        })
    }
}

impl UserData for WmInputPointer {
    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("position", |_, this, _: ()| this.position());
        methods.add_method("buttons", |_, this, _: ()| this.buttons());
    }
}

lua_typedef!(Pointer => WmInputPointer {
    fn position() -> Point;
    fn buttons() -> table<MouseButton>;
});

pub struct PointerButtonEvent {
	pub serial: Serial,
	pub utime: u64,
	pub button: u32,
	pub state: ButtonState,
    pub pointer: PointerHandle<Hazel>,
    pub default_prevented: RefCell<bool>,
}

impl PointerButtonEvent {
    pub fn name() -> String {
        String::from("pointer_button")
    }

    pub fn button_event(&self) -> ButtonEvent {
        ButtonEvent {
            serial: self.serial,
            time: (self.utime / 1000) as u32,
            button: self.button,
            state: self.state,
        }
    }
}

impl UserData for PointerButtonEvent {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("button", |_, this| Ok(LuaMouseButton(this.button)));
        fields.add_field_method_get("state", |_, this| Ok(format!("{:?}", this.state)));
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

lua_typedef!(PointerButtonEvent => PointerButtonEvent {
    let button: MouseButton;
    let state: string;
    let pointer: Pointer;
    fn prevent_default() -> nil;
});

#[derive(Clone)]
pub struct PointerMoveEvent {
    pub delta: Point<f64, Logical>,
    pub delta_unaccel: Point<f64, Logical>,
    pub position: Point<f64, Logical>,
    pub output_position: Option<Point<f64, Logical>>,
    pub pointer: PointerHandle<Hazel>,
    pub utime: u64,
    pub default_prevented: RefCell<bool>,
    pub serial: Serial,
}

impl PointerMoveEvent {
    pub fn name() -> String {
        String::from("pointer_move")
    }

    pub fn time_msec(&self) -> u32 {
        (self.utime / 1000) as u32
    }

    pub fn motion(&self) -> MotionEvent {
        MotionEvent {
            location: self.position,
            serial: self.serial,
            time: self.time_msec(),
        }
    }

    pub fn relative_motion(&self) -> RelativeMotionEvent {
        RelativeMotionEvent {
            delta: self.delta,
            delta_unaccel: self.delta_unaccel,
            utime: self.utime,
        }
    }
}

impl UserData for PointerMoveEvent {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("delta", |_, this| Ok(LuaPoint(this.delta)));
        fields.add_field_method_get("delta_unaccel", |_, this| Ok(LuaPoint(this.delta_unaccel)));
        fields.add_field_method_get("position", |_, this| Ok(LuaPoint(this.position)));
        fields.add_field_method_get("output_position", |_, this| {
            Ok(this.output_position.map(LuaPoint))
        });
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

lua_typedef!(PointerMoveEvent => PointerMoveEvent {
    let delta: Point;
    let delta_unaccel: Point;
    let position: Point;
    let output_position: Option<Point>;
    let pointer: Pointer;
    fn prevent_default() -> nil;
});
