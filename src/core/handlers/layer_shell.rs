use smithay::{
    backend::renderer::utils::with_renderer_surface_state,
    desktop::{LayerMap, LayerSurface, PopupKind, WindowSurfaceType, layer_map_for_output},
    output::{Output, WeakOutput},
    reexports::wayland_server::protocol::{wl_output::WlOutput, wl_surface::WlSurface},
    utils::{Logical, Rectangle},
    wayland::{
        compositor::with_states,
        shell::wlr_layer::{Layer, LayerSurfaceData, WlrLayerShellHandler, WlrLayerShellState},
    },
};
use tracing::info;

use smithay::reexports::wayland_server::Resource;

use std::cell::Cell;

use crate::{core::{Hazel, layer_properties::LayerProperties}, lua::api::wm_layers::LayerHandle};

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
        layer.user_data().insert_if_missing(|| LayerOutput(output.downgrade()));
        let _ = layer_map_for_output(&output).map_layer(&layer);

        let _ = self
            .wm()
            .layers
            .events
            .emit("new_surface".to_owned(), LayerHandle(layer));
    }

    fn layer_destroyed(&mut self, surface: smithay::wayland::shell::wlr_layer::LayerSurface) {
        info!("Layer surface destroyed");

        let wl_surface = surface.wl_surface().clone();
        let outputs: Vec<_> = self.compositor.space.outputs().cloned().collect();
        for output in &outputs {
            let layer = {
                let map = layer_map_for_output(output);
                map.layers()
                    .find(|l| l.layer_surface().wl_surface() == &wl_surface)
                    .cloned()
            };
            if let Some(layer) = layer {
                layer_map_for_output(output).unmap_layer(&layer);
                let _ = self
                    .wm()
                    .layers
                    .events
                    .emit("remove_surface".to_owned(), LayerHandle(layer));
                self.emit_layers_changed(output);
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
        self.compositor.unconstrain_popup(&popup);
        let _ = self
            .compositor
            .smithay
            .popups
            .track_popup(PopupKind::Xdg(popup));
    }
}


pub struct LayerOutput(pub WeakOutput);

#[derive(Clone, Copy, PartialEq)]
struct LayerSnapshot {
    properties: LayerProperties,
    geometry: Option<Rectangle<i32, Logical>>,
}

impl LayerSnapshot {
    fn of(map: &LayerMap, layer: &LayerSurface) -> Self {
        Self { properties: LayerProperties::of(layer), geometry: map.layer_geometry(layer) }
    }
}

struct LayerSnapshotLuaSaw(Cell<LayerSnapshot>);

impl Hazel {
    pub fn emit_layer_mapped(&mut self, output: &Output, surface: &WlSurface) {
        let Some(layer) = layer_map_for_output(output).layer_for_surface(surface, WindowSurfaceType::TOPLEVEL).cloned() else {
            return;
        };
        let has_buffer = with_renderer_surface_state(layer.wl_surface(), |state| {
            state.buffer().is_some()
        })
        .unwrap_or(false);
        if has_buffer
            && layer.user_data().insert_if_missing(|| {
                LayerSnapshotLuaSaw(Cell::new(LayerSnapshot::of(&layer_map_for_output(output), &layer)))
            })
        {
            let _ = self
                .wm()
                .layers
                .events
                .emit("mapped".to_owned(), LayerHandle(layer));
        }
    }

    pub fn emit_layers_changed(&mut self, output: &Output) {
        let changed: Vec<LayerSurface> = {
            let map = layer_map_for_output(output);
            map.layers()
                .filter(|layer| {
                    layer.user_data().get::<LayerSnapshotLuaSaw>().is_some_and(|last| {
                        let now = LayerSnapshot::of(&map, layer);
                        last.0.replace(now) != now
                    })
                })
                .cloned()
                .collect()
        };
        for layer in changed {
            let _ = self
                .wm()
                .layers
                .events
                .emit("changed".to_owned(), LayerHandle(layer));
        }
    }
}

pub fn handle_commit(output: &smithay::output::Output, surface: &WlSurface) -> bool {
    let mut map = layer_map_for_output(output);

    let Some(layer) = map
        .layer_for_surface(surface, smithay::desktop::WindowSurfaceType::TOPLEVEL)
        .cloned()
    else {
        return false;
    };

    map.arrange();

    let initial_configure_sent = with_states(surface, |states| {
        states
            .data_map
            .get::<LayerSurfaceData>()
            .map(|data| crate::core::locking::LockRecovering::lock_recovering(data).initial_configure_sent)
            .unwrap_or(false)
    });

    if !initial_configure_sent {
        layer.layer_surface().send_configure();
    }

    true
}
