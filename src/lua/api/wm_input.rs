use mlua::{FromLua, IntoLua, UserData, Value};

use crate::{
    core::{GlobalHazel, focus::FocusTarget},
    impl_lua_event_handler, impl_lua_event_source,
    lua::{
        api::{
            utils::LuaPoint, wm_input_seats::WmInputSeats, wm_layers::LayerHandle,
            wm_windows::WmWindow,
        },
        event_handler::LuaEventHandler,
    },
    lua_typedef,
};

#[derive(Default)]
pub struct WmInput {
    pub events: LuaEventHandler,
}

impl_lua_event_source!(WmInput);

pub struct LuaFocusTarget(pub FocusTarget);

impl IntoLua for LuaFocusTarget {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<Value> {
        match self.0 {
            FocusTarget::Window(window) => WmWindow(window).into_lua(lua),
            FocusTarget::Layer(layer) => LayerHandle(layer).into_lua(lua),
        }
    }
}

impl FromLua for LuaFocusTarget {
    fn from_lua(value: Value, _: &mlua::Lua) -> mlua::Result<Self> {
        if let Value::UserData(ud) = &value {
            if let Ok(window) = ud.borrow::<WmWindow>() {
                return Ok(Self(FocusTarget::Window(window.0.clone())));
            }
            if let Ok(layer) = ud.borrow::<LayerHandle>() {
                return Ok(Self(FocusTarget::Layer(layer.0.clone())));
            }
        }
        Err(mlua::Error::FromLuaConversionError {
            from: value.type_name(),
            to: String::from("Window or Layer"),
            message: Some(String::from("expected a window or a layer surface")),
        })
    }
}

impl UserData for WmInput {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("seats", |_, _| Ok(WmInputSeats));
    }

    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        impl_lua_event_handler!(methods);

        methods.add_method("target_under", |_, _, point: LuaPoint<f64, _>| {
            GlobalHazel::try_with(|hazel| {
                Ok(hazel.compositor.target_under(point.0).map(LuaFocusTarget))
            })
        });
    }
}

lua_typedef!(WmInput => WmInput {
    let seats: WmSeats;
    use key => KeyEvent;
    use shortcuts_inhibit_request => ShortcutsInhibitor;
    use pointer_move => PointerMoveEvent;
    use pointer_button => PointerButtonEvent;
    use pointer_button_forwarded => PointerButtonEvent;
    use pointer_constraint => PointerConstraint;
    use pointer_gesture => PointerGestureEvent;
    use new_keyboard => Keyboard;
    use new_pointer => Pointer;
    fn target_under(point: Point) -> Option<FocusTarget>;
});
