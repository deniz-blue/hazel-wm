use crate::core::shell_window::ShellWindow;
use smithay::{
    desktop::layer_map_for_output,
    output::{Mode, Output, PhysicalProperties, Scale, Subpixel},
    reexports::wayland_server::backend::GlobalId,
    utils::{Logical, Point, Size, Transform},
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    wayland::{
        compositor::{TraversalAction, with_surface_tree_downward},
        fractional_scale::with_fractional_scale,
    },
};
use tracing::info;

use crate::{
    backend::{Backend, ModeChange},
    core::{Hazel, compositor::HazelCompositor},
    lua::api::{wm_layers::LayerHandle, wm_outputs::WmOutputHandle},
};

const MAXIMUM_VIRTUAL_OUTPUT_SIZE: i32 = 16384;

struct OutputGlobal(GlobalId);
struct VirtualOutput;

pub fn is_virtual_output(output: &Output) -> bool {
    output.user_data().get::<VirtualOutput>().is_some()
}

pub fn mark_virtual_output(output: &Output) {
    output.user_data().insert_if_missing(|| VirtualOutput);
}

impl HazelCompositor {
    pub fn position_after_last_output(&self) -> Point<i32, Logical> {
        let right_edge = self
            .space
            .outputs()
            .filter_map(|output| self.space.output_geometry(output))
            .map(|geometry| geometry.loc.x.saturating_add(geometry.size.w))
            .max()
            .unwrap_or(0);
        Point::from((right_edge, 0))
    }

    pub fn virtual_outputs(&self) -> Vec<Output> {
        self.space.outputs().filter(|output| is_virtual_output(output)).cloned().collect()
    }
}

impl Hazel {
    pub fn register_output(&mut self, output: &Output, position: Option<Point<i32, Logical>>) {
        info!("Output added: {}", output.name());
        let global = output.create_global::<Hazel>(&self.display_handle);
        output.user_data().insert_if_missing(|| OutputGlobal(global));
        let position = position.unwrap_or_else(|| self.compositor.position_after_last_output());
        output.change_current_state(None, None, None, Some(position));
        self.compositor.space.map_output(output, position);
        layer_map_for_output(output).arrange();
        self.compositor.refresh_fractional_scales();
    }

    pub fn unregister_output(&mut self, output: &Output) {
        info!("Output removed: {}", output.name());
        let _ = self.wm().outputs.events.emit("removed".to_owned(), WmOutputHandle::new(output));
        for request in self.take_capture_requests(output) {
            self.fail_capture(request);
        }
        let layers: Vec<_> = layer_map_for_output(output).layers().cloned().collect();
        for layer in layers {
            layer.layer_surface().send_close();
            layer_map_for_output(output).unmap_layer(&layer);
            let _ = self.wm().layers.events.emit("remove_surface".to_owned(), LayerHandle(layer));
        }
        self.compositor.drawing.remove_on_output(output);
        self.compositor.space.unmap_output(output);
        if let Some(OutputGlobal(global)) = output.user_data().get::<OutputGlobal>() {
            self.display_handle.remove_global::<Hazel>(global.clone());
        }
        self.compositor.refresh_fractional_scales();
    }

    pub fn add_virtual_output(&mut self, name: &str, size: Size<i32, Logical>) -> mlua::Result<Output> {
        if !matches!(self.backend, Backend::Winit(_)) {
            return Err(mlua::Error::external("virtual outputs exist only on the nested backend"));
        }
        if !(1..=MAXIMUM_VIRTUAL_OUTPUT_SIZE).contains(&size.w) || !(1..=MAXIMUM_VIRTUAL_OUTPUT_SIZE).contains(&size.h) {
            return Err(mlua::Error::external(format!("a virtual output is between 1x1 and {0}x{0}, got {1}x{2}", MAXIMUM_VIRTUAL_OUTPUT_SIZE, size.w, size.h)));
        }
        if self.compositor.space.outputs().any(|output| output.name() == name) {
            return Err(mlua::Error::external(format!("there is already an output named {name:?}")));
        }
        let output = Output::new(
            name.to_owned(),
            PhysicalProperties { size: (0, 0).into(), subpixel: Subpixel::Unknown, make: "Hazel".into(), model: "Virtual".into(), serial_number: String::new() },
        );
        let mode = Mode { size: (size.w, size.h).into(), refresh: 60_000 };
        output.change_current_state(Some(mode), Some(Transform::Normal), None, None);
        output.set_preferred(mode);
        mark_virtual_output(&output);
        self.register_output(&output, None);
        let _ = self.wm().outputs.events.emit("added".to_owned(), WmOutputHandle::new(&output));
        Ok(output)
    }

    pub fn remove_virtual_output(&mut self, output: &Output) -> Result<(), String> {
        if !is_virtual_output(output) {
            return Err(format!("{} is not a virtual output", output.name()));
        }
        self.unregister_output(output);
        Ok(())
    }

    pub fn set_output_scale(&mut self, output: &Output, scale: f64) {
        output.change_current_state(None, None, Some(Scale::Fractional(scale)), None);
        self.output_geometry_changed(output);
    }

    pub fn set_output_transform(&mut self, output: &Output, transform: Transform) {
        output.change_current_state(None, Some(transform), None, None);
        self.output_geometry_changed(output);
    }

    pub fn set_output_mode(&mut self, output: &Output, mode: Mode) -> Result<(), String> {
        match self.backend.request_output_mode(output, mode)? {
            ModeChange::Applied => self.output_geometry_changed(output),
            ModeChange::Requested => {}
        }
        Ok(())
    }

    pub fn output_geometry_changed(&mut self, output: &Output) {
        layer_map_for_output(output).arrange();
        self.compositor.refresh_fractional_scales();
        let _ = self
            .wm()
            .outputs
            .events
            .emit("changed".to_owned(), WmOutputHandle::new(output));
        self.emit_layers_changed(output);
    }
}

impl HazelCompositor {
    pub fn preferred_scale_for(&self, surface: &WlSurface) -> f64 {
        let output = self
            .window_of_surface(surface)
            .and_then(|window| self.output_of_window(&window))
            .or_else(|| self.layer_output_of_surface(surface))
            .or_else(|| self.space.outputs().next().cloned());
        output.map_or(1.0, |output| output.current_scale().fractional_scale())
    }

    fn layer_output_of_surface(&self, surface: &WlSurface) -> Option<Output> {
        self.space
            .outputs()
            .find(|output| {
                layer_map_for_output(output)
                    .layer_for_surface(surface, smithay::desktop::WindowSurfaceType::TOPLEVEL)
                    .is_some()
            })
            .cloned()
    }

    pub fn refresh_fractional_scales(&self) {
        let toplevels = self
            .windows()
            .filter_map(|window| window.root_surface());
        let layers = self.space.outputs().flat_map(|output| {
            layer_map_for_output(output)
                .layers()
                .map(|layer| layer.wl_surface().clone())
                .collect::<Vec<_>>()
        });
        for root in toplevels.chain(layers) {
            let scale = self.preferred_scale_for(&root);
            with_surface_tree_downward(
                &root,
                (),
                |_, _, _| TraversalAction::DoChildren(()),
                |_, states, _| {
                    with_fractional_scale(states, |state| state.set_preferred_scale(scale));
                },
                |_, _, _| true,
            );
        }
    }
}
