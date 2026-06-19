use smithay::{
    backend::renderer::utils::on_commit_buffer_handler,
    desktop::{WindowSurfaceType, layer_map_for_output},
    reexports::wayland_server::{Client, Resource, protocol::wl_surface::WlSurface},
    wayland::compositor::{
        CompositorClientState, CompositorHandler, CompositorState, get_parent, is_sync_subsurface,
    },
};

use crate::core::{
    Hazel,
    client_state::ClientState,
    handlers::{layer_shell, xdg_shell},
};

impl CompositorHandler for Hazel {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor.smithay.compositor_state
    }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        &client.get_data::<ClientState>().unwrap().compositor_state
    }

    fn commit(&mut self, surface: &WlSurface) {
        on_commit_buffer_handler::<Self>(surface);
        if !is_sync_subsurface(surface) {
            let mut root = surface.clone();
            while let Some(parent) = get_parent(&root) {
                root = parent;
            }
            if let Some(window) = self
                .compositor
                .space
                .elements()
                .find(|w| w.toplevel().unwrap().wl_surface() == &root)
            {
                window.on_commit();
            }
        };

        xdg_shell::handle_commit(
            &mut self.compositor.smithay.popups,
            &self.compositor.space,
            surface,
        );
        // resize_grab::handle_commit(&mut self.compositor.space, surface);

        // Handle layer-surface commits — find which output this surface belongs to.
        for output in self.compositor.space.outputs() {
            let map = layer_map_for_output(output);
            if map
                .layer_for_surface(surface, WindowSurfaceType::TOPLEVEL)
                .is_some()
            {
                drop(map);
                layer_shell::handle_commit(output, surface);
                break;
            }
            drop(map);
        }

        self.wm()
            .events
            .emit("commit".to_string(), surface.id().protocol_id())
            .unwrap();
    }
}

smithay::delegate_compositor!(Hazel);
