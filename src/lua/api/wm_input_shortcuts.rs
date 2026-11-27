use mlua::UserData;
use smithay::wayland::keyboard_shortcuts_inhibit::KeyboardShortcutsInhibitor;

use crate::{core::GlobalHazel, lua::api::wm_windows::WmWindow, lua_typedef};

#[derive(Clone)]
pub struct WmShortcutsInhibitor(pub KeyboardShortcutsInhibitor);

impl WmShortcutsInhibitor {
    pub fn name() -> String {
        String::from("shortcuts_inhibit_request")
    }
}

impl UserData for WmShortcutsInhibitor {
    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("activate", |_, this, ()| {
            this.0.activate();
            Ok(())
        });
        methods.add_method("deactivate", |_, this, ()| {
            this.0.inactivate();
            Ok(())
        });
        methods.add_method("is_active", |_, this, ()| Ok(this.0.is_active()));
        methods.add_method("window", |_, this, ()| {
            GlobalHazel::try_with(|hazel| {
                Ok(hazel.compositor.window_of_surface(this.0.wl_surface()).map(WmWindow))
            })
        });
    }
}

lua_typedef!(ShortcutsInhibitor => WmShortcutsInhibitor {
    fn activate() -> nil;
    fn deactivate() -> nil;
    fn is_active() -> boolean;
    fn window() -> Option<Window>;
});
