use smithay::{
    output::Output,
    reexports::wayland_server::protocol::{wl_output::WlOutput, wl_surface::WlSurface},
    utils::SERIAL_COUNTER,
    wayland::session_lock::{LockSurface, SessionLockHandler, SessionLockManagerState, SessionLocker},
};

use crate::core::Hazel;

impl SessionLockHandler for Hazel {
    fn lock_state(&mut self) -> &mut SessionLockManagerState {
        &mut self.compositor.smithay.session_lock_state
    }

    fn lock(&mut self, locker: SessionLocker) {
        let session_lock = &mut self.compositor.session_lock;
        if session_lock.is_active() && !session_lock.is_orphaned() {
            return;
        }
        session_lock.begin(locker);
        self.focus_lock_surface();
        self.confirm_lock_when_covered();
    }

    fn unlock(&mut self) {
        self.compositor.session_lock.release();
        self.clear_keyboard_focus_of_every_seat();
        self.emit_session_event("session_unlocked");
    }

    fn new_surface(&mut self, surface: LockSurface, wl_output: WlOutput) {
        let Some(output) = Output::from_resource(&wl_output) else {
            return;
        };
        if let Some(geometry) = self.compositor.space.output_geometry(&output) {
            surface.with_pending_state(|state| state.size = Some((geometry.size.w as u32, geometry.size.h as u32).into()));
        }
        self.compositor.session_lock.add_surface(surface, output);
        self.focus_lock_surface();
    }
}

impl Hazel {
    fn emit_session_event(&mut self, name: &str) {
        let _ = self.wm().events.emit(name.to_owned(), ());
    }

    pub fn lock_surface_committed(&mut self, surface: &WlSurface) {
        if self.compositor.session_lock.owns_surface(surface) {
            self.focus_lock_surface();
            self.confirm_lock_when_covered();
        }
    }

    fn confirm_lock_when_covered(&mut self) {
        if !self.compositor.session_lock.is_waiting_for_surfaces() || !self.every_output_shows_a_lock_surface() {
            return;
        }
        if self.compositor.session_lock.confirm() {
            self.emit_session_event("session_locked");
        }
    }

    fn every_output_shows_a_lock_surface(&self) -> bool {
        self.compositor.space.outputs().all(|output| {
            self.compositor
                .session_lock
                .surface_on(output)
                .is_some_and(|surface| smithay::backend::renderer::utils::with_renderer_surface_state(surface.wl_surface(), |state| state.buffer().is_some()).unwrap_or(false))
        })
    }

    fn focus_lock_surface(&mut self) {
        let Some(surface) = self.compositor.session_lock.focus_surface() else {
            return;
        };
        let seats: Vec<_> = self.compositor.seats.values().cloned().collect();
        for seat in seats {
            if let Some(keyboard) = seat.get_keyboard() {
                keyboard.set_focus(self, Some(surface.clone()), SERIAL_COUNTER.next_serial());
            }
        }
    }
}

