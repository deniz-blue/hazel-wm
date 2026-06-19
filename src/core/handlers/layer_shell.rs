use smithay::{
    delegate_layer_shell,
    desktop::{layer_map_for_output, LayerSurface, PopupKind},
    output::Output,
    reexports::wayland_server::protocol::{wl_output::WlOutput, wl_surface::WlSurface},
    wayland::{
        compositor::with_states,
        shell::wlr_layer::{
            Layer, LayerSurfaceData, WlrLayerShellHandler, WlrLayerShellState,
        },
    },
};
use tracing::info;

use smithay::reexports::wayland_server::Resource;

use crate::core::Hazel;

impl WlrLayerShellHandler for Hazel {
    fn shell_state(&mut self) -> &mut WlrLayerShellState {
        &mut self.compositor.smithay.layer_shell_state
    }

    fn new_layer_surface(
        &mut self,
        surface: smithay::wayland::shell::wlr_layer::LayerSurface,
        wl_output: Option<WlOutput>,
        _layer: Layer,
        namespace: String,
    ) {
        // Resolve the output: use the specified one, or fall back to the first available.
        let output = wl_output
            .as_ref()
            .and_then(Output::from_resource)
            .or_else(|| self.compositor.space.outputs().next().cloned());

        let Some(output) = output else {
            info!("No output available for layer surface, closing");
            surface.send_close();
            return;
        };

        info!(
            "New layer surface: namespace={:?}, output={}",
            namespace,
            output.name()
        );

        let layer = LayerSurface::new(surface, namespace);
        let _ = layer_map_for_output(&output).map_layer(&layer);
    }

    fn layer_destroyed(&mut self, surface: smithay::wayland::shell::wlr_layer::LayerSurface) {
        info!("Layer surface destroyed");

        let wl_surface = surface.wl_surface().clone();

        // Find and unmap from the output's LayerMap.
        // Collect outputs first to avoid borrowing issues with layer_map_for_output.
        let outputs: Vec<_> = self.compositor.space.outputs().cloned().collect();
        for output in &outputs {
            let layer = {
                let map = layer_map_for_output(output);
                map
                    .layers()
                    .find(|l| l.layer_surface().wl_surface() == &wl_surface)
                    .cloned()
            };
            if let Some(layer) = layer {
                let mut map = layer_map_for_output(output);
                map.unmap_layer(&layer);
                break;
            }
        }
    }

    fn new_popup(
        &mut self,
        _parent: smithay::wayland::shell::wlr_layer::LayerSurface,
        popup: smithay::wayland::shell::xdg::PopupSurface,
    ) {
        info!("New layer-shell popup: {:?}", popup.wl_surface().id());
        let _ = self
            .compositor
            .smithay
            .popups
            .track_popup(PopupKind::Xdg(popup));
    }
}

delegate_layer_shell!(Hazel);

/// Handle a layer-shell surface commit.
///
/// Should be called from `CompositorHandler::commit()` for surfaces that belong
/// to a layer-shell surface. Returns `true` if the surface was a layer surface.
pub fn handle_commit(
    output: &smithay::output::Output,
    surface: &WlSurface,
) -> bool {
    let mut map = layer_map_for_output(output);

    let Some(layer) = map
        .layer_for_surface(surface, smithay::desktop::WindowSurfaceType::TOPLEVEL)
        .cloned()
    else {
        return false;
    };

    // Arrange the layers to compute positions/sizes.
    map.arrange();

    // Send initial configure if not yet sent.
    let initial_configure_sent = with_states(surface, |states| {
        states
            .data_map
            .get::<LayerSurfaceData>()
            .map(|data| data.lock().unwrap().initial_configure_sent)
            .unwrap_or(false)
    });

    if !initial_configure_sent {
        layer.layer_surface().send_configure();
    }

    true
}
