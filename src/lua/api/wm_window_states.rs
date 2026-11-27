use mlua::UserData;
use smithay::{
    desktop::Window,
    reexports::wayland_protocols::xdg::shell::server::xdg_toplevel,
};

use crate::{core::shell_window::ShellWindow, lua_typedef};

pub struct WindowStates(pub Window);


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
        if let Some(state) = state_from_name(name) {
            self.0.set_shell_state(state, enabled);
        }
    }

    pub fn has(&self, name: &str) -> bool {
        state_from_name(name).is_some_and(|state| self.0.has_shell_state(state))
    }

    pub fn list(&self) -> Vec<String> {
        self.0
            .shell_states()
            .iter()
            .map(|state| state_name(state).to_string())
            .collect()
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

