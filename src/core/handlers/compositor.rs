use smithay::{
    backend::renderer::utils::on_commit_buffer_handler,
    reexports::wayland_server::{Client, protocol::wl_surface::WlSurface},
    wayland::compositor::{
        CompositorClientState, CompositorHandler, CompositorState, get_parent, is_sync_subsurface,
    },
};

use crate::core::{Hazel, client_state::ClientState, handlers::xdg_shell};

impl CompositorHandler for Hazel {
    fn compositor_state<'a>(&'a mut self) -> &'a mut CompositorState {
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

        xdg_shell::handle_commit(&mut self.compositor.smithay.popups, &self.compositor.space, surface);
        // resize_grab::handle_commit(&mut self.compositor.space, surface);
    }
}

smithay::delegate_compositor!(Hazel);
