use smithay::{
    desktop::{Window, WindowSurface},
    reexports::{
        wayland_protocols::{
            xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode as DecorationMode,
            xdg::shell::server::xdg_toplevel::State,
        },
        wayland_server::protocol::wl_surface::WlSurface,
    },
    utils::{Logical, Size},
    wayland::{
        compositor::with_states,
        seat::WaylandFocus,
        shell::xdg::{ToplevelSurface, XdgToplevelSurfaceData},
    },
};

pub trait ShellWindow {
    fn root_surface(&self) -> Option<WlSurface>;
    fn is_shell_alive(&self) -> bool;
    fn shell_title(&self) -> Option<String>;
    fn shell_app_id(&self) -> Option<String>;
    fn request_close(&self);
    fn has_shell_state(&self, state: State) -> bool;
    fn set_shell_state(&self, state: State, enabled: bool);
    fn shell_states(&self) -> Vec<State>;
    fn set_requested_size(&self, size: Option<Size<i32, Logical>>);
    fn set_decoration_mode(&self, mode: Option<DecorationMode>);
    fn send_pending_configure(&self);
}

fn toplevel_of(window: &Window) -> &ToplevelSurface {
    let WindowSurface::Wayland(toplevel) = window.underlying_surface();
    toplevel
}

impl ShellWindow for Window {
    fn root_surface(&self) -> Option<WlSurface> {
        self.wl_surface().map(|surface| surface.into_owned())
    }

    fn is_shell_alive(&self) -> bool {
        toplevel_of(self).alive()
    }

    fn shell_title(&self) -> Option<String> {
        let toplevel = toplevel_of(self);
        if !toplevel.alive() {
            return None;
        }
        with_states(toplevel.wl_surface(), |states| {
            let data = states.data_map.get::<XdgToplevelSurfaceData>()?;
            data.lock().ok()?.title.clone()
        })
    }

    fn shell_app_id(&self) -> Option<String> {
        let toplevel = toplevel_of(self);
        if !toplevel.alive() {
            return None;
        }
        with_states(toplevel.wl_surface(), |states| {
            let data = states.data_map.get::<XdgToplevelSurfaceData>()?;
            data.lock().ok()?.app_id.clone()
        })
    }

    fn request_close(&self) {
        toplevel_of(self).send_close();
    }

    fn has_shell_state(&self, state: State) -> bool {
        toplevel_of(self).with_pending_state(|pending| pending.states.contains(state))
    }

    fn set_shell_state(&self, state: State, enabled: bool) {
        toplevel_of(self).with_pending_state(|pending| {
            if enabled {
                pending.states.set(state);
            } else {
                pending.states.unset(state);
            }
        });
    }

    fn shell_states(&self) -> Vec<State> {
        toplevel_of(self).with_pending_state(|pending| pending.states.clone().into_iter().collect())
    }

    fn set_requested_size(&self, size: Option<Size<i32, Logical>>) {
        toplevel_of(self).with_pending_state(|pending| pending.size = size);
    }

    fn set_decoration_mode(&self, mode: Option<DecorationMode>) {
        toplevel_of(self).with_pending_state(|pending| pending.decoration_mode = mode);
    }

    fn send_pending_configure(&self) {
        toplevel_of(self).send_pending_configure().into_iter().for_each(drop);
    }
}
