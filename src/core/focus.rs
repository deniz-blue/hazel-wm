use smithay::{
    input::Seat,
    desktop::{
        LayerSurface, Window, WindowSurface, WindowSurfaceType, find_popup_root_surface, layer_map_for_output,
    },
    reexports::wayland_server::{Client, Resource, protocol::wl_surface::WlSurface},
    utils::{Logical, Point, SERIAL_COUNTER},
    wayland::{compositor::get_parent, shell::xdg::ToplevelSurface},
};

use crate::{
    core::{Hazel, compositor::HazelCompositor, shell_window::ShellWindow},
    lua::api::{wm_input_seats::WmSeat, wm_windows::WmWindow},
};

#[derive(Clone, PartialEq)]
pub enum FocusTarget {
    Window(Window),
    Layer(LayerSurface),
}

impl FocusTarget {
    pub fn surface(&self) -> Option<WlSurface> {
        match self {
            FocusTarget::Window(window) => window.root_surface(),
            FocusTarget::Layer(layer) => Some(layer.wl_surface().clone()),
        }
    }
}

pub fn subsurface_root(surface: &WlSurface) -> WlSurface {
    let mut root = surface.clone();
    while let Some(parent) = get_parent(&root) {
        root = parent;
    }
    root
}

impl HazelCompositor {
    pub fn shell_root_of(&self, surface: &WlSurface) -> WlSurface {
        let root = subsurface_root(surface);
        self.smithay
            .popups
            .find_popup(&root)
            .and_then(|popup| find_popup_root_surface(&popup).ok())
            .unwrap_or(root)
    }

    pub fn window_of_surface(&self, surface: &WlSurface) -> Option<Window> {
        let root = self.shell_root_of(surface);
        self.window_with_root_surface(&root).cloned()
    }

    pub fn parent_of_window(&self, window: &Window) -> Option<Window> {
        let WindowSurface::Wayland(toplevel) = window.underlying_surface();
        self.window_of_surface(&toplevel.parent()?)
    }

    pub fn layer_of_surface(&self, surface: &WlSurface) -> Option<LayerSurface> {
        let root = self.shell_root_of(surface);
        self.space.outputs().find_map(|output| {
            layer_map_for_output(output)
                .layer_for_surface(&root, WindowSurfaceType::TOPLEVEL)
                .cloned()
        })
    }

    pub fn focus_target_of_surface(&self, surface: &WlSurface) -> Option<FocusTarget> {
        self.window_of_surface(surface)
            .map(FocusTarget::Window)
            .or_else(|| self.layer_of_surface(surface).map(FocusTarget::Layer))
    }

    pub fn target_under(&self, pos: Point<f64, Logical>) -> Option<FocusTarget> {
        let (surface, _) = self.input_under(pos)?;
        self.focus_target_of_surface(&surface)
    }

    pub fn keyboard_focus_target_of(&self, seat: &Seat<Hazel>) -> Option<FocusTarget> {
        let surface = seat.get_keyboard()?.current_focus()?;
        self.focus_target_of_surface(&surface)
    }

    pub fn seats_focusing_window(&self, window: &Window) -> Vec<Seat<Hazel>> {
        let mut seats: Vec<_> = self
            .seats
            .values()
            .filter(|seat| self.keyboard_focus_target_of(seat) == Some(FocusTarget::Window(window.clone())))
            .cloned()
            .collect();
        seats.sort_by(|a, b| a.name().cmp(b.name()));
        seats
    }

    pub fn shortcuts_inhibited(&self, keyboard: &smithay::input::keyboard::KeyboardHandle<Hazel>) -> bool {
        use smithay::wayland::keyboard_shortcuts_inhibit::KeyboardShortcutsInhibitorSeat;
        self.seats
            .values()
            .find(|seat| seat.get_keyboard().as_ref() == Some(keyboard))
            .is_some_and(|seat| seat.keyboard_shortcuts_inhibited())
    }

    pub fn topmost_window(&self) -> Option<Window> {
        self.space.elements().last().cloned()
    }
}

impl Hazel {
    pub fn window_for_toplevel(&self, toplevel: &ToplevelSurface) -> Option<Window> {
        self.compositor.window_of_surface(toplevel.wl_surface())
    }

    pub fn set_keyboard_focus(&mut self, seat: &Seat<Hazel>, target: Option<&FocusTarget>) {
        if self.compositor.session_lock.is_active() {
            return;
        }
        let surface = match target {
            Some(FocusTarget::Window(window)) if !self.compositor.is_window_visible(window) => {
                return;
            }
            Some(target) => match target.surface() {
                Some(surface) => Some(surface),
                None => return,
            },
            None => None,
        };
        let Some(keyboard) = seat.get_keyboard() else { return };
        let had_focus = keyboard.current_focus().is_some();
        keyboard.set_focus(self, surface.clone(), SERIAL_COUNTER.next_serial());
        if surface.is_none() && had_focus {
            self.apply_keyboard_focus(seat, None);
        }
    }

    pub fn clear_keyboard_focus_of_every_seat(&mut self) {
        let seats: Vec<_> = self.compositor.seats.values().cloned().collect();
        for seat in seats {
            self.set_keyboard_focus(&seat, None);
        }
    }

    pub fn emit_pending_focus_changes(&mut self) {
        while !self.compositor.pending_focus_changes.is_empty() {
            let changes = std::mem::take(&mut self.compositor.pending_focus_changes);
            for (seat_name, window) in changes {
                let Some(seat) = self.compositor.seats.get(&seat_name).cloned() else { continue };
                let _ = self
                    .wm()
                    .windows
                    .events
                    .emit("focus".to_owned(), (window.map(WmWindow), WmSeat(seat)));
            }
        }
    }

    pub fn client_of(&self, surface: &WlSurface) -> Option<Client> {
        self.display_handle.get_client(surface.id()).ok()
    }
}
