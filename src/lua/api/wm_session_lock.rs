use mlua::UserData;
use smithay::{output::Output, wayland::session_lock::LockSurface};

use crate::{
    core::{GlobalHazel, window_borders::parse_hex_color},
    lua::api::wm_outputs::WmOutputHandle,
    lua_typedef,
};

#[derive(Default)]
pub struct WmSessionLock;

impl UserData for WmSessionLock {
    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("state", |_, _, ()| {
            GlobalHazel::try_with(|hazel| Ok(hazel.compositor.session_lock.phase_name()))
        });

        methods.add_method("surfaces", |_, _, ()| {
            GlobalHazel::try_with(|hazel| {
                let surfaces: Vec<_> = hazel
                    .compositor
                    .session_lock
                    .live_surfaces()
                    .map(|(surface, output)| WmLockSurface { surface: surface.clone(), output: output.clone() })
                    .collect();
                Ok(surfaces)
            })
        });

        methods.add_method("set_backdrop_color", |_, _, color: String| {
            let rgba = parse_hex_color(&color)
                .ok_or_else(|| mlua::Error::external(format!("{color:?} is not a #rrggbb or #rrggbbaa colour")))?;
            GlobalHazel::try_with(|hazel| {
                hazel.compositor.session_lock.set_backdrop_color(rgba);
                Ok(())
            })
        });
    }
}

lua_typedef!(WmSessionLock => WmSessionLock {
    fn state() -> string;
    fn surfaces() -> Array<LockSurface>;
    fn set_backdrop_color(color: string) -> nil;
});

#[derive(Clone)]
pub struct WmLockSurface {
    surface: LockSurface,
    output: Output,
}

impl UserData for WmLockSurface {
    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("output", |_, this, ()| Ok(WmOutputHandle::new(&this.output)));
        methods.add_method("has_buffer", |_, this, ()| {
            Ok(smithay::backend::renderer::utils::with_renderer_surface_state(this.surface.wl_surface(), |state| {
                state.buffer().is_some()
            })
            .unwrap_or(false))
        });
    }
}

lua_typedef!(LockSurface => WmLockSurface {
    fn output() -> Output;
    fn has_buffer() -> boolean;
});
