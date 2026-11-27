use std::{cell::Cell, rc::Rc};

use mlua::UserData;
use smithay::{
    backend::input::{AxisSource, ButtonState},
    input::{
        keyboard::ModifiersState,
        pointer::{ButtonEvent, MotionEvent, PointerHandle, RelativeMotionEvent},
    },
    utils::{Logical, Point, Serial},
};

use crate::{
    core::{
        GlobalHazel, Hazel,
        input::{pointer_grab::holds_exclusive_grab, scroll::Scroll},
    },
    lua::api::{
        wm_input_gestures::parse_gesture,
        utils::LuaPoint,
        wm_input_keyboard::ModifiersStateUserData,
        wm_input_seats::WmSeat,
        wm_input_sym::LuaMouseButton,
        wm_windows::WmWindow,
    },
    lua_typedef,
};

pub struct WmInputPointer(pub PointerHandle<Hazel>);

impl WmInputPointer {
    pub fn position(&self) -> Result<LuaPoint<f64, Logical>, mlua::Error> {
        Ok(LuaPoint(self.0.current_location()))
    }

    pub fn buttons(&self) -> Result<Vec<u32>, mlua::Error> {
        GlobalHazel::try_with(|hazel| {
            Ok(hazel
                .compositor
                .pointers
                .get(&self.0)
                .map(|s| s.pressed.clone())
                .unwrap_or_default())
        })
    }
}

impl UserData for WmInputPointer {
    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("position", |_, this, _: ()| this.position());
        methods.add_method("buttons", |_, this, _: ()| this.buttons());
        methods.add_method("window_under", |_, this, _: ()| {
            GlobalHazel::with(|hazel| {
                hazel
                    .compositor
                    .window_under(this.0.current_location())
                    .map(|(window, _)| WmWindow(window.clone()))
            })
        });
        methods.add_method("move", |_, this, position: LuaPoint<f64, _>| {
            GlobalHazel::try_with(|hazel| {
                hazel
                    .move_pointer(&this.0, position.0, 0)
                    .map_err(|e| mlua::Error::external(e.to_string()))
            })
        });

        methods.add_method(
            "button",
            |_, this, (button, state): (LuaMouseButton, String)| {
                let state = match state.as_str() {
                    "Pressed" | "pressed" => ButtonState::Pressed,
                    "Released" | "released" => ButtonState::Released,
                    other => {
                        return Err(mlua::Error::external(format!(
                            "button state must be \"pressed\" or \"released\", got {other:?}"
                        )));
                    }
                };
                GlobalHazel::try_with(|hazel| {
                    hazel
                        .press_pointer_button(&this.0, button.0, state, 0)
                        .map_err(|e| mlua::Error::external(e.to_string()))
                })
            },
        );

        methods.add_method(
            "scroll",
            |_, this, (horizontal, vertical, source): (f64, f64, Option<String>)| {
                let scroll = match source.as_deref().unwrap_or("wheel") {
                    "wheel" => Scroll::wheel_notches(horizontal, vertical),
                    "finger" => Scroll::pixels(AxisSource::Finger, horizontal, vertical),
                    "continuous" => Scroll::pixels(AxisSource::Continuous, horizontal, vertical),
                    other => {
                        return Err(mlua::Error::external(format!(
                            "scroll source must be \"wheel\", \"finger\" or \"continuous\", got {other:?}"
                        )));
                    }
                };
                GlobalHazel::try_with(|hazel| {
                    hazel.send_scroll(&this.0, scroll, 0);
                    Ok(())
                })
            },
        );

        methods.add_method(
            "gesture",
            |_, this, (kind, phase, options): (String, String, Option<mlua::Table>)| {
                let gesture = parse_gesture(&kind, &phase, options)?;
                GlobalHazel::try_with(|hazel| {
                    hazel.send_gesture(&this.0, gesture, 0).map_err(mlua::Error::external)
                })
            },
        );

        methods.add_method("grab", |_, this, _: ()| {
            GlobalHazel::try_with(|hazel| {
                hazel.grab_pointer_exclusively(&this.0);
                Ok(())
            })
        });

        methods.add_method("ungrab", |_, this, _: ()| {
            GlobalHazel::try_with(|hazel| {
                hazel.release_pointer_grab(&this.0);
                Ok(())
            })
        });

        methods.add_method("grabbed", |_, this, _: ()| Ok(holds_exclusive_grab(&this.0)));

        methods.add_method("seat", |_, this, _: ()| {
            GlobalHazel::try_with(|hazel| {
                Ok(hazel
                    .compositor
                    .seats
                    .values()
                    .find(|seat| seat.get_pointer().map(|p| p == this.0).unwrap_or(false))
                    .map(|s| WmSeat(s.clone())))
            })
        });
    }
}

lua_typedef!(Pointer => WmInputPointer {
    fn seat() -> Option<Seat>;
    fn position() -> Point;
    fn buttons() -> Array<integer>;
    fn window_under() -> Option<Window>;
    fn scroll(horizontal: number, vertical: number, source: string) -> nil;
    fn gesture(kind: string, phase: string, options: table) -> nil;
    fn grab() -> nil;
    fn ungrab() -> nil;
    fn grabbed() -> boolean;
});

#[derive(Clone)]
pub struct PointerButtonEvent {
    pub serial: Serial,
    pub utime: u64,
    pub button: u32,
    pub state: ButtonState,
    pub pointer: PointerHandle<Hazel>,
    pub modifiers: ModifiersState,
    pub default_prevented: Rc<Cell<bool>>,
}

impl PointerButtonEvent {
    pub fn name() -> String {
        String::from("pointer_button")
    }

    pub fn forwarded_name() -> String {
        String::from("pointer_button_forwarded")
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
        fields.add_field_method_get("modifiers", |_, this| {
            Ok(ModifiersStateUserData(this.modifiers))
        });
    }

    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("prevent_default", |_, this, _: ()| {
            this.default_prevented.set(true);
            Ok(())
        });
    }
}

lua_typedef!(PointerButtonEvent => PointerButtonEvent {
    let button: MouseButton;
    let state: string;
    let pointer: Pointer;
    let modifiers: Modifiers;
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
    pub modifiers: ModifiersState,
    pub default_prevented: Rc<Cell<bool>>,
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
        fields.add_field_method_get("modifiers", |_, this| {
            Ok(ModifiersStateUserData(this.modifiers))
        });
    }

    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("prevent_default", |_, this, _: ()| {
            this.default_prevented.set(true);
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
    let modifiers: Modifiers;
    fn prevent_default() -> nil;
});
