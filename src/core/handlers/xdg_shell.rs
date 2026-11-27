use smithay::{
    desktop::{PopupKind, Window},
    input::Seat,
    output::Output,
    reexports::{
        wayland_protocols::xdg::shell::server::xdg_toplevel,
        wayland_server::{
            Resource,
            protocol::{wl_output::WlOutput, wl_seat, wl_surface::WlSurface},
        },
    },
    utils::{SERIAL_COUNTER, Serial},
    wayland::{
        shell::xdg::{
            PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState,
        },
    },
};
use tracing::{error, info};

use crate::{
    core::{Hazel, compositor::HazelCompositor},
    lua::api::{
        wm_outputs::WmOutputHandle, wm_windows::WmWindow,
    },
};

impl XdgShellHandler for Hazel {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.compositor.smithay.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        info!("New toplevel: {:?}", surface.wl_surface().id());
        self.open_window(Window::new_wayland_window(surface));
    }

    fn toplevel_destroyed(&mut self, surface: ToplevelSurface) {
        let Some(window) = self.window_for_toplevel(&surface) else {
            return;
        };
        self.close_window(window);
    }

    fn maximize_request(&mut self, surface: ToplevelSurface) {
        self.emit_window_event("maximize_request", &surface);
    }

    fn unmaximize_request(&mut self, surface: ToplevelSurface) {
        self.emit_window_event("unmaximize_request", &surface);
    }

    fn fullscreen_request(&mut self, surface: ToplevelSurface, output: Option<WlOutput>) {
        let Some(window) = self.window_for_toplevel(&surface) else {
            return;
        };
        let output = output
            .as_ref()
            .and_then(Output::from_resource)
            .map(|output| WmOutputHandle::new(&output));
        let _ = self.wm().windows.events.emit(
            "fullscreen_request".to_owned(),
            (WmWindow(window), output),
        );
    }

    fn unfullscreen_request(&mut self, surface: ToplevelSurface) {
        self.emit_window_event("unfullscreen_request", &surface);
    }

    fn minimize_request(&mut self, surface: ToplevelSurface) {
        self.emit_window_event("minimize_request", &surface);
    }

    fn parent_changed(&mut self, surface: ToplevelSurface) {
        self.emit_window_event("parent_changed", &surface);
    }

    fn title_changed(&mut self, surface: ToplevelSurface) {
        self.emit_window_event("title_changed", &surface);
    }

    fn app_id_changed(&mut self, surface: ToplevelSurface) {
        self.emit_window_event("app_id_changed", &surface);
    }

    fn new_popup(&mut self, surface: PopupSurface, _positioner: PositionerState) {
        info!("New popup: {:?}", surface.wl_surface().id());

        self.compositor.unconstrain_popup(&surface);
        let _ = self
            .compositor
            .smithay
            .popups
            .track_popup(PopupKind::Xdg(surface));
    }

    fn reposition_request(
        &mut self,
        surface: PopupSurface,
        positioner: PositionerState,
        token: u32,
    ) {
        info!(
            "Reposition request for popup: {:?}",
            surface.wl_surface().id()
        );
        surface.with_pending_state(|state| {
            let geometry = positioner.get_geometry();
            state.geometry = geometry;
            state.positioner = positioner;
        });
        self.compositor.unconstrain_popup(&surface);
        surface.send_repositioned(token);
    }

    fn move_request(&mut self, surface: ToplevelSurface, seat: wl_seat::WlSeat, serial: Serial) {
        self.emit_drag_request(&surface, &seat, serial, None);
    }

    fn resize_request(
        &mut self,
        surface: ToplevelSurface,
        seat: wl_seat::WlSeat,
        serial: Serial,
        edges: xdg_toplevel::ResizeEdge,
    ) {
        self.emit_drag_request(&surface, &seat, serial, Some(resize_edges_name(edges)));
    }

    fn grab(&mut self, surface: PopupSurface, seat: wl_seat::WlSeat, serial: Serial) {
        self.grab_popup(surface, &seat, serial);
    }

    fn popup_destroyed(&mut self, surface: PopupSurface) {
        let Some(parent) = surface.get_parent_surface() else { return };
        let root = self.compositor.shell_root_of(&parent);
        self.compositor.smithay.popups.cleanup();
        let seats: Vec<_> = self.compositor.seats.values().cloned().collect();
        for seat in seats {
            let Some(keyboard) = seat.get_keyboard() else { continue };
            if keyboard.current_focus().as_ref() == Some(surface.wl_surface()) {
                keyboard.set_focus(self, Some(root.clone()), SERIAL_COUNTER.next_serial());
            }
        }
    }
}

fn resize_edges_name(edges: xdg_toplevel::ResizeEdge) -> &'static str {
    use xdg_toplevel::ResizeEdge::*;
    match edges {
        Top => "top",
        Bottom => "bottom",
        Left => "left",
        Right => "right",
        TopLeft => "top_left",
        TopRight => "top_right",
        BottomLeft => "bottom_left",
        BottomRight => "bottom_right",
        _ => "none",
    }
}

impl Hazel {
    fn emit_drag_request(
        &self,
        surface: &ToplevelSurface,
        seat: &wl_seat::WlSeat,
        serial: Serial,
        edges: Option<&'static str>,
    ) {
        let Some(pointer) = Seat::<Hazel>::from_resource(seat).and_then(|s| s.get_pointer())
        else {
            return;
        };
        let Some(window) = self.window_for_toplevel(surface) else {
            return;
        };
        if !pointer.has_grab(serial) {
            return;
        }
        self.emit_window_drag_request(window, pointer, edges);
    }

    fn emit_window_event(&self, name: &str, surface: &ToplevelSurface) {
        if let Some(window) = self.window_for_toplevel(surface) {
            let _ = self
                .wm()
                .windows
                .events
                .emit(name.to_owned(), WmWindow(window));
        }
    }
}

/// Should be called on `WlSurface::commit`
pub fn handle_commit(compositor: &mut HazelCompositor, surface: &WlSurface) {
    // Handle toplevel commits.
    if let Some(window) = compositor
        .window_with_root_surface(surface)
        .cloned()
        && let Some(toplevel) = window.toplevel()
        && !toplevel.is_initial_configure_sent()
    {
        toplevel.send_configure();
    }

    // Handle popup commits.
    let popups = &mut compositor.smithay.popups;
    popups.commit(surface);
    if let Some(popup) = popups.find_popup(surface) {
        match popup {
            PopupKind::Xdg(ref xdg) => {
                if !xdg.is_initial_configure_sent() {
                    if let Err(e) = xdg.send_configure() {
                        error!("Initial popup configure failed: {e:?}");
                    }
                }
            }
            PopupKind::InputMethod(ref _input_method) => {}
        }
    }
}

