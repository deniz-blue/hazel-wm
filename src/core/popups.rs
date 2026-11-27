use crate::core::shell_window::ShellWindow;
use smithay::{
    desktop::{
        PopupKeyboardGrab, PopupKind, PopupPointerGrab, PopupUngrabStrategy, WindowSurfaceType,
        find_popup_root_surface, get_popup_toplevel_coords, layer_map_for_output,
    },
    input::{Seat, pointer::Focus},
    reexports::wayland_server::protocol::{wl_seat::WlSeat, wl_surface::WlSurface},
    utils::{Logical, Point, Rectangle, Serial},
    wayland::shell::xdg::PopupSurface,
};

use crate::core::{Hazel, compositor::HazelCompositor};

struct PopupRoot {
    origin: Point<i32, Logical>,
    bounds: Rectangle<i32, Logical>,
}

impl HazelCompositor {
    fn popup_root(&self, root: &WlSurface) -> Option<PopupRoot> {
        if let Some(window) = self
            .space
            .elements()
            .find(|w| w.root_surface().as_ref() == Some(root))
        {
            let bounds = self
                .space
                .outputs_for_element(window)
                .iter()
                .filter_map(|output| self.space.output_geometry(output))
                .reduce(|a, b| a.merge(b))?;
            let origin = self.space.element_geometry(window)?.loc;
            return Some(PopupRoot { origin, bounds });
        }

        self.space.outputs().find_map(|output| {
            let bounds = self.space.output_geometry(output)?;
            let map = layer_map_for_output(output);
            let layer = map.layer_for_surface(root, WindowSurfaceType::TOPLEVEL)?;
            let origin = map.layer_geometry(layer)?.loc + bounds.loc;
            Some(PopupRoot { origin, bounds })
        })
    }

    pub fn unconstrain_popup(&self, popup: &PopupSurface) {
        let kind = PopupKind::Xdg(popup.clone());
        let Ok(root) = find_popup_root_surface(&kind) else {
            return;
        };
        let Some(root) = self.popup_root(&root) else {
            return;
        };

        let mut target = root.bounds;
        target.loc -= get_popup_toplevel_coords(&kind) + root.origin;

        popup.with_pending_state(|state| {
            state.geometry = state.positioner.get_unconstrained_geometry(target);
        });
    }
}

impl Hazel {
    pub fn grab_popup(&mut self, popup: PopupSurface, seat: &WlSeat, serial: Serial) {
        let Some(seat) = Seat::<Hazel>::from_resource(seat) else {
            return;
        };
        let kind = PopupKind::Xdg(popup);
        let Ok(root) = find_popup_root_surface(&kind) else {
            return;
        };
        let Ok(mut grab) = self
            .compositor
            .smithay
            .popups
            .grab_popup(root, kind, &seat, serial)
        else {
            return;
        };

        if let Some(keyboard) = seat.get_keyboard() {
            let owned_by_other_grab = keyboard.is_grabbed()
                && !(keyboard.has_grab(serial)
                    || keyboard.has_grab(grab.previous_serial().unwrap_or(serial)));
            if owned_by_other_grab {
                grab.ungrab(PopupUngrabStrategy::All);
                return;
            }
            keyboard.set_focus(self, grab.current_grab(), serial);
            keyboard.set_grab(self, PopupKeyboardGrab::new(&grab), serial);
        }

        if let Some(pointer) = seat.get_pointer() {
            let owned_by_other_grab = pointer.is_grabbed()
                && !(pointer.has_grab(serial)
                    || pointer.has_grab(grab.previous_serial().unwrap_or_else(|| grab.serial())));
            if owned_by_other_grab {
                grab.ungrab(PopupUngrabStrategy::All);
                return;
            }
            pointer.set_grab(self, PopupPointerGrab::new(&grab), serial, Focus::Keep);
        }
    }
}
