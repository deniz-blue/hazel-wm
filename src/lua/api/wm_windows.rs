use mlua::UserData;
use smithay::{desktop::Window, utils::Logical};

use crate::{
    impl_lua_event_handler, impl_lua_event_source,
    lua::{
        api::utils::{LuaPoint, LuaSize},
        event_handler::LuaEventHandler,
    },
    lua_typedef, with_hazel,
};

#[derive(Default)]
pub struct WmWindows {
    pub events: LuaEventHandler,
}

impl_lua_event_source!(WmWindows);

impl UserData for WmWindows {
    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        impl_lua_event_handler!(methods);
    }
}

lua_typedef!(WmWindows => WmWindows {
    use new_window => Window;
});

pub struct WmWindow(pub Window);

impl WmWindow {
    pub fn get_position(&self) -> LuaPoint<i32, Logical> {
        with_hazel!(hazel => {
            LuaPoint(
                hazel
                    .compositor
                    .space
                    .element_location(&self.0)
                    .unwrap_or_default()
            )
        })
    }

    pub fn set_position(&self, position: LuaPoint<f64, Logical>) {
        with_hazel!(hazel => {
            hazel
                .compositor
                .space
                .map_element(self.0.clone(), position.0.to_i32_round(), true);
        })
    }

    pub fn get_size(&self) -> LuaSize<i32, Logical> {
        with_hazel!(hazel => {
            LuaSize(
                hazel
                    .compositor
                    .space
                    .element_geometry(&self.0)
                    .map(|g| g.size)
                    .unwrap_or_default()
            )
        })
    }

    pub fn set_size(&self, size: LuaSize<f64, Logical>) {
        if let Some(toplevel) = self.0.toplevel() {
            toplevel.with_pending_state(|s| s.size = Some(size.0.to_i32_round()));
        }
    }
}

impl UserData for WmWindow {
    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("position", |_, this, _: ()| Ok(this.get_position()));
        methods.add_method("set_position", |_, this, position: LuaPoint<_, _>| {
            Ok(this.set_position(position))
        });

        methods.add_method("size", |_, this, _: ()| Ok(this.get_size()));
        methods.add_method("set_size", |_, this, size: LuaSize<_, _>| {
            Ok(this.set_size(size))
        });
    }
}

lua_typedef!(Window => WmWindow {
    fn position() -> Point;
    fn set_position(position: Point) -> nil;
    fn size() -> Size;
    fn set_size(size: Size) -> nil;
});
