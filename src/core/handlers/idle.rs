use smithay::{
    reexports::wayland_server::{Resource, protocol::wl_surface::WlSurface},
    wayland::{
        idle_inhibit::IdleInhibitHandler,
        idle_notify::{IdleNotifierHandler, IdleNotifierState},
    },
};

use crate::core::Hazel;

impl IdleNotifierHandler for Hazel {
    fn idle_notifier_state(&mut self) -> &mut IdleNotifierState<Self> {
        &mut self.compositor.smithay.idle_notifier_state
    }
}

impl IdleInhibitHandler for Hazel {
    fn inhibit(&mut self, surface: WlSurface) {
        self.compositor.idle_inhibitors.push(surface);
        self.update_idle_inhibition();
    }

    fn uninhibit(&mut self, surface: WlSurface) {
        self.compositor.idle_inhibitors.retain(|s| s != &surface);
        self.update_idle_inhibition();
    }
}

impl Hazel {
    pub fn update_idle_inhibition(&mut self) {
        self.compositor.idle_inhibitors.retain(|surface| surface.is_alive());
        let inhibited = !self.compositor.idle_inhibitors.is_empty();
        self.compositor
            .smithay
            .idle_notifier_state
            .set_is_inhibited(inhibited);
    }

    pub fn notify_idle_activity(&mut self) {
        let seats: Vec<_> = self.compositor.seats.values().cloned().collect();
        for seat in seats {
            self.compositor.smithay.idle_notifier_state.notify_activity(&seat);
        }
    }
}

