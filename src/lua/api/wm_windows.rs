use std::sync::atomic::{AtomicU64, Ordering};

use mlua::UserData;
use smithay::{
    desktop::Window,
    reexports::{
        wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode as DecorationMode,
    },
    utils::{Logical, Size},
};

use crate::{
    core::{
        GlobalHazel,
        shell_window::ShellWindow,
        window_borders::{BorderStyle, MAXIMUM_BORDER_WIDTH, parse_hex_color, set_window_border},
        window_layers::{WindowLayer, set_window_layer, window_layer},
        window_mapping::is_mapped,
    },
    impl_lua_event_handler, impl_lua_event_source,
    lua::{
        api::{
            utils::{LuaPoint, LuaSize},
            wm_input_seats::WmSeat,
            shape_spec::ShapeSpec,
            wm_outputs::WmOutputHandle,
            wm_shapes::{WmShape, add_window_shape},
            wm_window_states::WindowStates,
        },
        event_handler::LuaEventHandler,
    },
    lua_eq, lua_fromlua_userdataonly, lua_typedef, with_hazel,
};

struct WindowId(u64);

impl WindowId {
    fn next() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

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
            |_, _, _: ()| GlobalHazel::try_with(|hazel| Ok(hazel.compositor.all_windows().len())),
        );

        methods.add_method("topmost", |_, _, _: ()| {
            GlobalHazel::try_with(|hazel| Ok(hazel.compositor.topmost_window().map(WmWindow)))
        });

        methods.add_method("by_id", |_, _, id: u64| {
            GlobalHazel::try_with(|hazel| Ok(hazel
                .compositor
                .windows()
                .find(|window| WmWindow(Window::clone(window)).id() == id)
                .cloned()
                .map(WmWindow)))
        });

        methods.add_method("get", |_, _, index: usize| {
            GlobalHazel::try_with(|hazel| {
                let idx = index.checked_sub(1);
                Ok(idx.and_then(|i| {
                    hazel.compositor.all_windows().into_iter().nth(i).map(WmWindow)
                }))
            })
        });

        methods.add_method("all", |_, _, _: ()| {
            GlobalHazel::try_with(|hazel| {
                Ok(hazel
                    .compositor
                    .all_windows()
                    .into_iter()
                    .map(WmWindow)
                    .collect::<Vec<_>>())
            })
        });

        methods.add_method("under", |_, _, point: LuaPoint<f64, _>| {
            GlobalHazel::try_with(|hazel| {
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
    use mapped => Window;
    use resized => Window;
    use closed => Window;
    use title_changed => Window;
    use app_id_changed => Window;
    use maximize_request => Window;
    use unmaximize_request => Window;
    use unfullscreen_request => Window;
    use minimize_request => Window;
    use unminimize_request => Window;
    use parent_changed => Window;
    use decoration_request => Window;
    use move_request => WindowDragRequest;
    use resize_request => WindowDragRequest;
    event focus(window: Option<Window>, seat: Seat);
    fn topmost() -> Option<Window>;
    fn count() -> number;
    fn by_id(id: integer) -> Option<Window>;
    fn get(index: number) -> Option<Window>;
    fn all() -> Array<Window>;
    fn under(point: Point) -> Option<Window>;
});

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WmWindow(pub Window);

impl WmWindow {
    pub fn get_position(&self) -> mlua::Result<LuaPoint<i32, Logical>> {
        with_hazel!(hazel => LuaPoint(hazel.compositor.window_location(&self.0)))
    }

    pub fn set_position(&self, position: LuaPoint<f64, Logical>) -> mlua::Result<()> {
        with_hazel!(hazel => {
            hazel
                .compositor
                .place_window(&self.0, position.0.to_i32_round());
        })
    }

    pub fn get_size(&self) -> LuaSize<i32, Logical> {
        LuaSize(self.0.geometry().size)
    }

    pub fn set_size(&self, size: LuaSize<f64, Logical>) {
        self.configure_size(Some(size.0.to_i32_round()));
    }

    pub fn clear_size(&self) {
        self.configure_size(None);
    }

    fn configure_size(&self, size: Option<Size<i32, Logical>>) {
        self.0.set_requested_size(size);
    }

    pub fn title(&self) -> Option<String> {
        self.0.shell_title()
    }

    pub fn app_id(&self) -> Option<String> {
        self.0.shell_app_id()
    }

    pub fn close(&self) {
        self.0.request_close();
    }

    pub fn set_activated(&self, active: bool) {
        self.0.set_activated(active);
    }

    pub fn raise(&self) -> mlua::Result<()> {
        with_hazel!(hazel => hazel.compositor.raise_window(&self.0))
    }

    pub fn id(&self) -> u64 {
        self.0.user_data().get_or_insert(WindowId::next).0
    }

    pub fn parent(&self) -> mlua::Result<Option<WmWindow>> {
        with_hazel!(hazel => hazel.compositor.parent_of_window(&self.0).map(WmWindow))
    }

    pub fn output(&self) -> mlua::Result<Option<WmOutputHandle>> {
        with_hazel!(hazel => hazel
            .compositor
            .output_of_window(&self.0)
            .map(|output| WmOutputHandle::new(&output)))
    }

    pub fn mapped(&self) -> bool {
        is_mapped(&self.0)
    }

    pub fn layer(&self) -> &'static str {
        window_layer(&self.0).name()
    }

    pub fn set_layer(&self, name: &str) -> Result<(), mlua::Error> {
        let layer = WindowLayer::from_name(name).ok_or_else(|| {
            mlua::Error::external(format!(
                "layer must be \"below\", \"normal\" or \"above\", got {name:?}"
            ))
        })?;
        set_window_layer(&self.0, layer);
        with_hazel!(hazel => hazel.compositor.restack())
    }

    pub fn set_border(&self, width: i32, color: &str) -> Result<(), mlua::Error> {
        let color = parse_hex_color(color).ok_or_else(|| {
            mlua::Error::external(format!(
                "border color must look like \"#rrggbb\" or \"#rrggbbaa\", got {color:?}"
            ))
        })?;
        if width > MAXIMUM_BORDER_WIDTH {
            return Err(mlua::Error::external(format!(
                "border width must be at most {MAXIMUM_BORDER_WIDTH}, got {width}"
            )));
        }
        set_window_border(&self.0, Some(BorderStyle { width: width.max(0), color }));
        Ok(())
    }

    pub fn clear_border(&self) {
        set_window_border(&self.0, None);
    }

    pub fn set_decoration(&self, mode: &str) -> Result<(), mlua::Error> {
        let mode = match mode {
            "server" => Some(DecorationMode::ServerSide),
            "client" => Some(DecorationMode::ClientSide),
            "none" => None,
            other => {
                return Err(mlua::Error::external(format!(
                    "decoration must be \"server\", \"client\" or \"none\", got {other:?}"
                )));
            }
        };
        self.0.set_decoration_mode(mode);
        Ok(())
    }

    pub fn focused_seats(&self) -> mlua::Result<Vec<WmSeat>> {
        with_hazel!(hazel => hazel
            .compositor
            .seats_focusing_window(&self.0)
            .into_iter()
            .map(WmSeat)
            .collect())
    }

    pub fn is_visible(&self) -> mlua::Result<bool> {
        with_hazel!(hazel => hazel.compositor.is_window_visible(&self.0))
    }

    pub fn set_visible(&self, visible: bool) -> mlua::Result<()> {
        with_hazel!(hazel => hazel.set_window_visible(&self.0, visible))
    }

    pub fn states(&self) -> WindowStates {
        WindowStates(self.0.clone())
    }
}

impl UserData for WmWindow {
    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("position", |_, this, _: ()| this.get_position());
        methods.add_method("set_position", |_, this, position: LuaPoint<_, _>| this.set_position(position));

        methods.add_method("size", |_, this, _: ()| Ok(this.get_size()));
        methods.add_method("clear_size", |_, this, _: ()| {
            this.clear_size();
            Ok(())
        });
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
        methods.add_method("focused_seats", |_, this, _: ()| this.focused_seats());
        methods.add_method("visible", |_, this, _: ()| this.is_visible());
        methods.add_method("set_visible", |_, this, visible: bool| this.set_visible(visible));
        methods.add_method("raise", |_, this, _: ()| this.raise());
        methods.add_method("kind", |_, _, _: ()| Ok("window"));
        methods.add_method("id", |_, this, _: ()| Ok(this.id()));
        methods.add_method("parent", |_, this, _: ()| this.parent());
        methods.add_method("output", |_, this, _: ()| this.output());
        methods.add_method("mapped", |_, this, _: ()| Ok(this.mapped()));
        methods.add_method("layer", |_, this, _: ()| Ok(this.layer()));
        methods.add_method("set_layer", |_, this, name: String| this.set_layer(&name));
        methods.add_method("set_border", |_, this, (width, color): (i32, String)| {
            this.set_border(width, &color)
        });
        methods.add_method("clear_border", |_, this, _: ()| {
            this.clear_border();
            Ok(())
        });
        methods.add_method("set_decoration", |_, this, mode: String| {
            this.set_decoration(&mode)
        });
        methods.add_method("add_shape", |_, this, spec: ShapeSpec| -> mlua::Result<WmShape> {
            add_window_shape(&this.0, spec)
        });

        lua_eq!(methods);
    }
}

lua_fromlua_userdataonly!(WmWindow);

lua_typedef!(Window => WmWindow {
    fn position() -> Point;
    fn set_position(position: Point) -> nil;
    fn size() -> Size;
    fn set_size(size: Size) -> nil;
    fn clear_size() -> nil;
    fn title() -> Option<string>;
    fn app_id() -> Option<string>;
    fn close() -> nil;
    fn set_activated(active: boolean) -> nil;
    fn states() -> WindowStates;
    fn focused_seats() -> Array<Seat>;
    fn visible() -> boolean;
    fn set_visible(visible: boolean) -> nil;
    fn raise() -> nil;
    fn kind() -> string;
    fn id() -> integer;
    fn parent() -> Option<Window>;
    fn output() -> Option<WmOutput>;
    fn mapped() -> boolean;
    fn layer() -> string;
    fn set_layer(layer: string) -> nil;
    fn set_border(width: integer, color: string) -> nil;
    fn clear_border() -> nil;
    fn set_decoration(mode: string) -> nil;
    fn add_shape(spec: ShapeSpec) -> Shape;
});
