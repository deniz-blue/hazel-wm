use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use smithay::{
    reexports::wayland_server::{
        Client, Resource,
        backend::{ClientData, ClientId, DisconnectReason},
        protocol::wl_surface::WlSurface,
    },
    wayland::compositor::{CompositorClientState, with_states},
};
use tracing::error;

const WL_DISPLAY_NO_MEMORY: u32 = 2;
pub const MAX_SURFACES_PER_CLIENT: usize = 8192;

#[derive(Default)]
pub struct ClientState {
    pub compositor_state: CompositorClientState,
    surfaces: Arc<AtomicUsize>,
}

struct SurfaceSlot(Arc<AtomicUsize>);

impl Drop for SurfaceSlot {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Relaxed);
    }
}

impl ClientState {
    fn reserve_surface(&self) -> Option<SurfaceSlot> {
        self.surfaces.fetch_add(1, Ordering::Relaxed);
        let slot = SurfaceSlot(self.surfaces.clone());
        (self.surfaces.load(Ordering::Relaxed) <= MAX_SURFACES_PER_CLIENT).then_some(slot)
    }
}

pub fn surface_created(surface: &WlSurface) {
    let Some(client) = surface.client() else { return };
    let Some(state) = client.get_data::<ClientState>() else { return };
    match state.reserve_surface() {
        Some(slot) => with_states(surface, |states| {
            states.data_map.insert_if_missing_threadsafe(|| slot);
        }),
        None => surface.post_error(
            WL_DISPLAY_NO_MEMORY,
            format!("a client cannot have more than {MAX_SURFACES_PER_CLIENT} surfaces"),
        ),
    }
}

impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {}
    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}
}

pub fn compositor_client_state(client: &Client) -> &CompositorClientState {
    client
        .get_data::<ClientState>()
        .map(|data| &data.compositor_state)
        .unwrap_or_else(|| {
            error!("a client has no Hazel client data, treating it as a new client");
            FALLBACK.with(|state| *state)
        })
}

thread_local! {
    static FALLBACK: &'static CompositorClientState = Box::leak(Box::default());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_client_can_have_the_limit_of_surfaces_and_no_more() {
        let state = ClientState::default();
        let slots: Vec<_> = (0..MAX_SURFACES_PER_CLIENT).map(|_| state.reserve_surface()).collect();
        assert!(slots.iter().all(Option::is_some));
        assert!(state.reserve_surface().is_none());
    }

    #[test]
    fn dropping_a_slot_frees_room_for_another_surface() {
        let state = ClientState::default();
        let slots: Vec<_> = (0..MAX_SURFACES_PER_CLIENT).map(|_| state.reserve_surface()).collect();
        drop(slots);
        let slot = state.reserve_surface();
        assert!(slot.is_some());
        assert_eq!(state.surfaces.load(Ordering::Relaxed), 1);
    }
}
