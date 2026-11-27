use std::cell::Cell;

use smithay::{
    backend::renderer::utils::with_renderer_surface_state,
    desktop::Window,
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::{Logical, Size},
};

use crate::{core::Hazel, lua::api::wm_windows::WmWindow};

pub fn is_mapped(window: &Window) -> bool {
    window.user_data().get_or_insert(Cell::<bool>::default).get()
}

#[derive(Default)]
struct SizeLuaKnows(Cell<Size<i32, Logical>>);

fn mark_mapped(window: &Window) -> bool {
    let mapped = window.user_data().get_or_insert(Cell::<bool>::default);
    !mapped.replace(true)
}

impl Hazel {
    pub fn emit_window_mapped(&self, surface: &WlSurface) {
        let has_buffer =
            with_renderer_surface_state(surface, |state| state.buffer().is_some()).unwrap_or(false);
        if !has_buffer {
            return;
        }
        let Some(window) = self.compositor.window_with_root_surface(surface) else {
            return;
        };
        if mark_mapped(window) {
            window.user_data().get_or_insert(SizeLuaKnows::default).0.set(window.geometry().size);
            let _ = self
                .wm()
                .windows
                .events
                .emit("mapped".to_owned(), WmWindow(window.clone()));
        }
    }

    pub fn emit_window_resized(&self, surface: &WlSurface) {
        let Some(window) = self.compositor.window_with_root_surface(surface) else {
            return;
        };
        if !is_mapped(window) {
            return;
        }
        let size = window.geometry().size;
        let known = window.user_data().get_or_insert(SizeLuaKnows::default);
        if known.0.replace(size) != size {
            let _ = self
                .wm()
                .windows
                .events
                .emit("resized".to_owned(), WmWindow(window.clone()));
        }
    }
}
