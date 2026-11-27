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
    client_state::{compositor_client_state, surface_created},
    handlers::{layer_shell, xdg_shell},
};

impl CompositorHandler for Hazel {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor.smithay.compositor_state
    }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        compositor_client_state(client)
    }

    fn new_surface(&mut self, surface: &WlSurface) {
        surface_created(surface);
    }

    fn commit(&mut self, surface: &WlSurface) {
        on_commit_buffer_handler::<Self>(surface);
        self.lock_surface_committed(surface);
        if !is_sync_subsurface(surface) {
            let mut root = surface.clone();
            while let Some(parent) = get_parent(&root) {
                root = parent;
            }
            if let Some(window) = self.compositor.window_with_root_surface(&root) {
                window.on_commit();
            }
        };

        xdg_shell::handle_commit(&mut self.compositor, surface);
        self.emit_window_mapped(surface);
        self.emit_window_resized(surface);
        // resize_grab::handle_commit(&mut self.compositor.space, surface);

        let layer_output = self
            .compositor
            .space
            .outputs()
            .find(|output| {
                layer_map_for_output(output)
                    .layer_for_surface(surface, WindowSurfaceType::TOPLEVEL)
                    .is_some()
            })
            .cloned();
        if let Some(output) = layer_output {
            layer_shell::handle_commit(&output, surface);
            self.emit_layer_mapped(&output, surface);
            self.emit_layers_changed(&output);
        }

        let _ = self
            .wm()
            .events
            .emit("commit".to_string(), surface.id().protocol_id());
    }
}

