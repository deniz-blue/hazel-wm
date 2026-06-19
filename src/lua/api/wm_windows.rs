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

        methods.add_method(
            "count",
            |_, _, _: ()| with_hazel!(hazel => Ok(hazel.compositor.space.elements().count())),
        );

        methods.add_method("get", |_, _, index: usize| {
            with_hazel!(hazel => {
                let idx = index.checked_sub(1);
                Ok(idx.and_then(|i| {
                    hazel
                        .compositor
                        .space
                        .elements()
                        .nth(i)
                        .cloned()
                        .map(WmWindow)
                }))
            })
        });

        methods.add_method("all", |_, _, _: ()| {
            with_hazel!(hazel => {
                Ok(hazel
                    .compositor
                    .space
                    .elements()
                    .cloned()
                    .map(WmWindow)
                    .collect::<Vec<_>>())
            })
        });

        methods.add_method("under", |_, _, point: LuaPoint<f64, _>| {
            with_hazel!(hazel => {
                let window = hazel
                    .compositor
                    .space
                    .element_under(point.0)
                    .map(|(w, _loc)| WmWindow(w.clone()));
                Ok(window)
            })
        });
    }
}

lua_typedef!(WmWindows => WmWindows {
    use new_window => Window;
    fn count() -> number;
    fn get(index: number) -> Window;
    fn all() -> Array<Window>;
    fn under(point: Point) -> Window;
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
                    .unwrap_or_default(),
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
                    .unwrap_or_default(),
            )
        })
    }

    pub fn set_size(&self, size: LuaSize<f64, Logical>) {
        if let Some(toplevel) = self.0.toplevel() {
            toplevel.with_pending_state(|s| s.size = Some(size.0.to_i32_round()));
        }
    }

    pub fn title(&self) -> Option<String> {
        use smithay::wayland::shell::xdg::XdgToplevelSurfaceData;

        self.0.toplevel().and_then(|toplevel| {
            smithay::wayland::compositor::with_states(toplevel.wl_surface(), |states| {
                states
                    .data_map
                    .get::<XdgToplevelSurfaceData>()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .title
                    .clone()
            })
        })
    }

    pub fn app_id(&self) -> Option<String> {
        use smithay::wayland::shell::xdg::XdgToplevelSurfaceData;

        self.0.toplevel().and_then(|toplevel| {
            smithay::wayland::compositor::with_states(toplevel.wl_surface(), |states| {
                states
                    .data_map
                    .get::<XdgToplevelSurfaceData>()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .app_id
                    .clone()
            })
        })
    }

    pub fn close(&self) {
        if let Some(toplevel) = self.0.toplevel() {
            toplevel.send_close();
        }
    }

    pub fn set_activated(&self, active: bool) {
        self.0.set_activated(active);
    }

    pub fn states(&self) -> WindowStates {
        WindowStates(self.0.clone())
    }
}

impl UserData for WmWindow {
    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("position", |_, this, _: ()| Ok(this.get_position()));
        methods.add_method("set_position", |_, this, position: LuaPoint<_, _>| {
            let _: () = this.set_position(position);
            Ok(())
        });

        methods.add_method("size", |_, this, _: ()| Ok(this.get_size()));
        methods.add_method("set_size", |_, this, size: LuaSize<_, _>| {
            let _: () = this.set_size(size);
            Ok(())
        });

        methods.add_method("title", |_, this, _: ()| Ok(this.title()));
        methods.add_method("app_id", |_, this, _: ()| Ok(this.app_id()));
        methods.add_method("close", |_, this, _: ()| {
            this.close();
            Ok(())
        });
        methods.add_method("set_activated", |_, this, active: bool| {
            this.set_activated(active);
            Ok(())
        });
        methods.add_method("states", |_, this, _: ()| Ok(this.states()));
    }
}

pub struct WindowStates(Window);

use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel;

fn state_from_name(name: &str) -> Option<xdg_toplevel::State> {
    match name {
        "maximized" => Some(xdg_toplevel::State::Maximized),
        "fullscreen" => Some(xdg_toplevel::State::Fullscreen),
        "resizing" => Some(xdg_toplevel::State::Resizing),
        "activated" => Some(xdg_toplevel::State::Activated),
        "tiled_left" => Some(xdg_toplevel::State::TiledLeft),
        "tiled_right" => Some(xdg_toplevel::State::TiledRight),
        "tiled_top" => Some(xdg_toplevel::State::TiledTop),
        "tiled_bottom" => Some(xdg_toplevel::State::TiledBottom),
        "suspended" => Some(xdg_toplevel::State::Suspended),
        _ => None,
    }
}

fn state_name(state: &xdg_toplevel::State) -> &'static str {
    match state {
        xdg_toplevel::State::Maximized => "maximized",
        xdg_toplevel::State::Fullscreen => "fullscreen",
        xdg_toplevel::State::Resizing => "resizing",
        xdg_toplevel::State::Activated => "activated",
        xdg_toplevel::State::TiledLeft => "tiled_left",
        xdg_toplevel::State::TiledRight => "tiled_right",
        xdg_toplevel::State::TiledTop => "tiled_top",
        xdg_toplevel::State::TiledBottom => "tiled_bottom",
        xdg_toplevel::State::Suspended => "suspended",
        _ => "unknown",
    }
}

impl WindowStates {
    /// Set or unset a toplevel state by name.
    ///
    /// Known state names: "maximized", "fullscreen", "resizing", "activated",
    /// "tiled_left", "tiled_right", "tiled_top", "tiled_bottom", "suspended".
    pub fn set(&self, name: &str, enabled: bool) {
        if let Some(state) = state_from_name(name)
            && let Some(toplevel) = self.0.toplevel()
        {
            toplevel.with_pending_state(|s| {
                if enabled {
                    s.states.set(state);
                } else {
                    s.states.unset(state);
                }
            });
            toplevel.send_configure();
        }
    }

    pub fn has(&self, name: &str) -> bool {
        if let Some(target) = state_from_name(name) {
            self.0
                .toplevel()
                .map(|toplevel| {
                    let state = toplevel.current_state();
                    state.states.contains(target)
                })
                .unwrap_or(false)
        } else {
            false
        }
    }

    pub fn list(&self) -> Vec<String> {
        self.0
            .toplevel()
            .map(|toplevel| {
                toplevel
                    .current_state()
                    .states
                    .into_iter()
                    .map(|s| state_name(&s).to_string())
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl UserData for WindowStates {
    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("set", |_, this, (name, enabled): (String, bool)| {
            this.set(&name, enabled);
            Ok(())
        });
        methods.add_method("has", |_, this, name: String| Ok(this.has(&name)));
        methods.add_method("list", |_, this, _: ()| Ok(this.list()));
    }
}

lua_typedef!(WindowStates => WindowStates {
    fn has(name: string) -> boolean;
    fn list() -> Array<string>;
    fn set(name: string, enabled: boolean) -> nil;
});

lua_typedef!(Window => WmWindow {
    fn position() -> Point;
    fn set_position(position: Point) -> nil;
    fn size() -> Size;
    fn set_size(size: Size) -> nil;
    fn title() -> string;
    fn app_id() -> string;
    fn close() -> nil;
    fn set_activated(active: boolean) -> nil;
    fn states() -> WindowStates;
    fn set_state(name: string, enabled: boolean) -> nil;
});
