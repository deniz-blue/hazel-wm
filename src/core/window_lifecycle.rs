use smithay::desktop::Window;

use crate::{core::Hazel, lua::api::wm_windows::WmWindow};

impl Hazel {
    pub fn open_window(&mut self, window: Window) {
        self.compositor
            .space
            .map_element(window.clone(), (0, 0), false);
        let _ = self
            .wm()
            .windows
            .events
            .emit(String::from("new_window"), WmWindow(window));
    }

    pub fn close_window(&mut self, window: Window) {
        self.compositor.forget_window(&window);
        let _ = self
            .wm()
            .windows
            .events
            .emit(String::from("closed"), WmWindow(window));
    }
}
