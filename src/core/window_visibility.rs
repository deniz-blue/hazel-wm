use smithay::{
    desktop::Window,
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::{Logical, Point},
};

use crate::core::{Hazel, compositor::HazelCompositor, shell_window::ShellWindow};

impl HazelCompositor {
    pub fn windows(&self) -> impl Iterator<Item = &Window> {
        self.space
            .elements()
            .chain(self.hidden_windows.iter().map(|(window, _)| window))
    }

    pub fn window_with_root_surface(&self, surface: &WlSurface) -> Option<&Window> {
        self.windows()
            .find(|window| window.root_surface().as_ref() == Some(surface))
    }

    pub fn all_windows(&self) -> Vec<Window> {
        self.windows().cloned().collect()
    }

    pub fn is_window_visible(&self, window: &Window) -> bool {
        self.space.elements().any(|w| w == window)
    }

    pub fn window_location(&self, window: &Window) -> Point<i32, Logical> {
        self.space
            .element_location(window)
            .or_else(|| {
                self.hidden_windows
                    .iter()
                    .find_map(|(w, location)| (w == window).then_some(*location))
            })
            .unwrap_or_default()
    }

    pub fn place_window(&mut self, window: &Window, location: Point<i32, Logical>) {
        if let Some(hidden) = self.hidden_windows.iter_mut().find(|(w, _)| w == window) {
            hidden.1 = location;
            return;
        }
        let above: Vec<Window> = self
            .space
            .elements()
            .skip_while(|w| *w != window)
            .skip(1)
            .cloned()
            .collect();
        self.space.map_element(window.clone(), location, false);
        for window in &above {
            self.space.raise_element(window, false);
        }
    }

    pub fn raise_window(&mut self, window: &Window) {
        self.space.raise_element(window, false);
        self.restack();
    }

    pub fn set_window_visible(&mut self, window: &Window, visible: bool) {
        if visible == self.is_window_visible(window) {
            return;
        }
        if visible {
            if let Some(index) = self.hidden_windows.iter().position(|(w, _)| w == window) {
                let (window, location) = self.hidden_windows.remove(index);
                self.space.map_element(window, location, false);
            }
        } else if let Some(location) = self.space.element_location(window) {
            self.space.unmap_elem(window);
            self.hidden_windows.push((window.clone(), location));
        }
    }

    pub fn forget_window(&mut self, window: &Window) {
        self.space.unmap_elem(window);
        self.hidden_windows.retain(|(w, _)| w != window);
        self.drawing.remove_on_window(window);
    }
}

impl Hazel {
    pub fn set_window_visible(&mut self, window: &Window, visible: bool) {
        let focusing_seats = self.compositor.seats_focusing_window(window);
        self.compositor.set_window_visible(window, visible);
        if !visible {
            for seat in focusing_seats {
                self.set_keyboard_focus(&seat, None);
            }
        }
    }
}
