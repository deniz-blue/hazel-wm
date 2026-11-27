use smithay::{desktop::Window, input::pointer::PointerHandle};

use crate::{
    core::Hazel,
    lua::api::{wm_window_requests::WindowDragRequest, wm_windows::WmWindow},
};

impl Hazel {
    pub fn emit_window_drag_request(
        &self,
        window: Window,
        pointer: PointerHandle<Hazel>,
        edges: Option<&'static str>,
    ) {
        let name = if edges.is_some() { "resize_request" } else { "move_request" };
        let request = WindowDragRequest { window: WmWindow(window), pointer, edges };
        let _ = self.wm().windows.events.emit(name.to_owned(), request);
    }

    pub fn pointer_with_pressed_button(&self) -> Option<PointerHandle<Hazel>> {
        self.compositor
            .pointers
            .iter()
            .find(|(_, state)| !state.pressed.is_empty())
            .map(|(pointer, _)| pointer.clone())
    }
}
