use smithay::{
    desktop::Window,
    wayland::foreign_toplevel_list::{ForeignToplevelListHandler, ForeignToplevelListState},
};

use crate::core::{Hazel, shell_window::ShellWindow, window_mapping::is_mapped};
use crate::lua::api::wm_windows::WmWindow;

impl ForeignToplevelListHandler for Hazel {
    fn foreign_toplevel_list_state(&mut self) -> &mut ForeignToplevelListState {
        &mut self.compositor.smithay.foreign_toplevel_list_state
    }
}

impl Hazel {
    pub fn sync_foreign_toplevel_list(&mut self) {
        if self.compositor.session_lock.is_active() {
            return;
        }
        let windows: Vec<Window> = self
            .compositor
            .windows()
            .filter(|window| is_mapped(window) && window.is_shell_alive())
            .cloned()
            .collect();
        let list = &mut self.compositor.smithay.foreign_toplevel_list_state;
        let listed = &mut self.compositor.listed_toplevels;

        listed.retain(|(window, handle)| {
            let open = windows.contains(window);
            if !open {
                list.remove_toplevel(handle);
            }
            open
        });
        for window in &windows {
            let wm_window = WmWindow(window.clone());
            let title = wm_window.title().unwrap_or_default();
            let app_id = wm_window.app_id().unwrap_or_default();
            match listed.iter().find(|(listed_window, _)| listed_window == window) {
                Some((_, handle)) => {
                    if handle.title() != title || handle.app_id() != app_id {
                        handle.send_title(&title);
                        handle.send_app_id(&app_id);
                        handle.send_done();
                    }
                }
                None => {
                    let handle = list.new_toplevel::<Hazel>(title, app_id);
                    listed.push((window.clone(), handle));
                }
            }
        }
    }
}

