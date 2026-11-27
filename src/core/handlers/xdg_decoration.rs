use smithay::{
    reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode,
    wayland::shell::xdg::{ToplevelSurface, decoration::XdgDecorationHandler},
};

use crate::{core::Hazel, lua::api::wm_windows::WmWindow};

impl Hazel {
    fn emit_decoration_request(&self, toplevel: &ToplevelSurface) {
        if let Some(window) = self.window_for_toplevel(toplevel) {
            let _ = self
                .wm()
                .windows
                .events
                .emit("decoration_request".to_owned(), WmWindow(window));
        }
    }
}

impl XdgDecorationHandler for Hazel {
    fn new_decoration(&mut self, toplevel: ToplevelSurface) {
        self.emit_decoration_request(&toplevel);
    }

    fn request_mode(&mut self, toplevel: ToplevelSurface, _mode: Mode) {
        self.emit_decoration_request(&toplevel);
    }

    fn unset_mode(&mut self, toplevel: ToplevelSurface) {
        self.emit_decoration_request(&toplevel);
    }
}

