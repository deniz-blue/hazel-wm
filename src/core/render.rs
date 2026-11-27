use std::{sync::OnceLock, time::Duration};

use smithay::{
    backend::{
        allocator::Fourcc,
        drm::{DrmNode, compositor::FrameFlags},
        renderer::{
            ImportAll, ImportMem, RendererSuper,
            element::{
                AsRenderElements, Kind,
                memory::{MemoryBuffer, MemoryRenderBuffer, MemoryRenderBufferRenderElement},
                solid::SolidColorRenderElement,
                surface::{WaylandSurfaceRenderElement, render_elements_from_surface_tree},
            },
        },
    },
    desktop::{layer_map_for_output, space::SurfaceTree, utils::send_frames_surface_tree},
    input::pointer::CursorImageStatus,
    reexports::{
        calloop::{
            LoopHandle,
            timer::{TimeoutAction, Timer},
        },
        drm::control::crtc::Handle as CrtcHandle,
    },
    reexports::wayland_protocols::xdg::shell::server::xdg_toplevel,
    utils::{IsAlive, Rectangle, Scale, Size, Transform},
    wayland::shell::wlr_layer::Layer,
};
use tracing::info;

use crate::{
    backend::Backend,
    core::{Hazel, compositor::HazelCompositor, window_borders::border_elements},
};

smithay::backend::renderer::element::render_elements! {
    pub HazelElement<R> where R: ImportAll + ImportMem;
    Space=WaylandSurfaceRenderElement<R>,
    Wrapped=smithay::backend::renderer::element::Wrap<WaylandSurfaceRenderElement<R>>,
    Memory=MemoryRenderBufferRenderElement<R>,
    Solid=SolidColorRenderElement,
}

const CURSOR_SIZE: i32 = 16;

fn cursor_render_buffer() -> &'static MemoryRenderBuffer {
    static BUF: OnceLock<MemoryRenderBuffer> = OnceLock::new();
    BUF.get_or_init(|| {
        let pixels = include_bytes!("../../assets/cursor.raw");
        let mem = MemoryBuffer::from_slice(pixels, Fourcc::Argb8888, (CURSOR_SIZE, CURSOR_SIZE));
        MemoryRenderBuffer::from_memory(mem, 1, Transform::Normal, None)
    })
}

impl HazelCompositor {
    pub fn cursor_elements<R>(
        &self,
        renderer: &mut R,
        output: &smithay::output::Output,
    ) -> Vec<HazelElement<R>>
    where
        R: ImportAll + ImportMem,
        <R as RendererSuper>::TextureId: Send + Clone + 'static,
    {
        let cursor_buf = cursor_render_buffer();
        let mut elements = Vec::new();

        let output_geometry = match self.space.output_geometry(output) {
            Some(g) => g,
            None => return elements,
        };

        let output_scale = Scale::from(output.current_scale().fractional_scale());

        for seat in self.seats.values() {
            let Some(pointer) = seat.get_pointer() else {
                continue;
            };
            let Some(state) = self.pointers.get(&pointer) else {
                continue;
            };

            let cursor_global = pointer.current_location();
            if !output_geometry.to_f64().contains(cursor_global) {
                continue;
            }

            let cursor_phys = (cursor_global - output_geometry.loc.to_f64())
                .to_physical(output_scale)
                .to_i32_round();

            match &state.cursor_status {
                CursorImageStatus::Surface(surf) if surf.alive() => {
                    let hotspot = smithay::wayland::compositor::with_states(surf, |states| {
                        states
                            .data_map
                            .get::<smithay::input::pointer::CursorImageSurfaceData>()
                            .map(|d| crate::core::locking::LockRecovering::lock_recovering(d).hotspot)
                    })
                    .unwrap_or_default();

                    let pos =
                        cursor_phys - hotspot.to_f64().to_physical(output_scale).to_i32_round();
                    elements.extend(
                        SurfaceTree::from_surface(surf).render_elements::<HazelElement<_>>(
                            renderer,
                            pos,
                            output_scale,
                            1.0,
                        ),
                    );
                }
                _ => {
                    let hotspot = CURSOR_SIZE / 2;
                    let pos = (
                        (cursor_phys.x - hotspot) as f64,
                        (cursor_phys.y - hotspot) as f64,
                    );
                    if let Ok(el) = MemoryRenderBufferRenderElement::from_buffer(
                        renderer,
                        pos,
                        cursor_buf,
                        None,
                        None,
                        Some(Size::from((CURSOR_SIZE, CURSOR_SIZE))),
                        Kind::Cursor,
                    ) {
                        elements.push(HazelElement::Memory(el));
                    }
                }
            }
        }

        elements
    }

    pub fn layer_elements<R>(
        &self,
        renderer: &mut R,
        output: &smithay::output::Output,
        layer: Layer,
    ) -> Vec<HazelElement<R>>
    where
        R: ImportAll + ImportMem,
        <R as RendererSuper>::TextureId: Clone + 'static,
    {
        let mut elements = Vec::new();

        let output_scale = Scale::from(output.current_scale().fractional_scale());

        let layer_map = layer_map_for_output(output);
        for surface in layer_map.layers_on(layer) {
            if !surface.alive() || !surface.wl_surface().alive() {
                continue;
            }
            let Some(geo) = layer_map.layer_geometry(surface) else {
                continue;
            };
            let pos = geo.loc.to_f64().to_physical(output_scale).to_i32_round();
            elements.extend(surface.render_elements::<HazelElement<_>>(
                renderer,
                pos,
                output_scale,
                1.0,
            ));
        }

        elements
    }

    pub fn window_elements<R>(
        &self,
        renderer: &mut R,
        output: &smithay::output::Output,
        fullscreen: bool,
    ) -> Vec<HazelElement<R>>
    where
        R: ImportAll + ImportMem,
        <R as RendererSuper>::TextureId: Send + Clone + 'static,
    {
        let Some(output_geometry) = self.space.output_geometry(output) else {
            return Vec::new();
        };
        let output_scale = Scale::from(output.current_scale().fractional_scale());

        self.space
            .elements()
            .rev()
            .filter(|window| {
                self.window_has_state(window, xdg_toplevel::State::Fullscreen) == fullscreen
            })
            .flat_map(|window| {
                let location = self.window_location(window);
                let origin = location - window.geometry().loc;
                let pos = (origin - output_geometry.loc)
                    .to_f64()
                    .to_physical(output_scale)
                    .to_i32_round();
                let content = window.render_elements::<HazelElement<R>>(renderer, pos, output_scale, 1.0);
                let shapes = self.window_shape_elements(renderer, window, location - output_geometry.loc, output_scale);
                let inner = Rectangle::new(location, window.geometry().size);
                let border = if inner.size.is_empty() {
                    Vec::new()
                } else {
                    border_elements(window, inner, output_geometry.loc, output_scale)
                };
                shapes
                    .in_front
                    .into_iter()
                    .chain(content)
                    .chain(border.into_iter().map(HazelElement::Solid))
                    .chain(shapes.behind)
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    pub fn send_frame_callbacks(&self, output: &smithay::output::Output, time: Duration) {
        if self.session_lock.is_active() {
            if let Some(lock_surface) = self.session_lock.surface_on(output) {
                send_frames_surface_tree(lock_surface.wl_surface(), output, time, Some(Duration::ZERO), |_, _| Some(output.clone()));
            }
            return;
        }
        for window in self.space.elements() {
            window.send_frame(output, time, Some(Duration::ZERO), |_, _| Some(output.clone()));
        }
        self.send_layer_frame_callbacks(output, time);
    }

    pub fn send_layer_frame_callbacks(&self, output: &smithay::output::Output, time: Duration) {
        for layer in layer_map_for_output(output).layers() {
            if layer.alive() {
                layer.send_frame(output, time, Some(Duration::ZERO), |_, _| Some(output.clone()));
            }
        }
    }

    fn locked_elements<R>(
        &self,
        renderer: &mut R,
        output: &smithay::output::Output,
    ) -> Vec<HazelElement<R>>
    where
        R: ImportAll + ImportMem,
        <R as RendererSuper>::TextureId: Send + Clone + 'static,
    {
        let Some(geometry) = self.space.output_geometry(output) else {
            return Vec::new();
        };
        let scale = Scale::from(output.current_scale().fractional_scale());
        let surface = self.session_lock.surface_on(output).map(|lock_surface| {
            render_elements_from_surface_tree(renderer, lock_surface.wl_surface(), (0, 0), scale, 1.0, Kind::Unspecified)
        });
        surface
            .into_iter()
            .flatten()
            .map(HazelElement::Space)
            .chain([HazelElement::Solid(self.session_lock.backdrop(output, geometry, scale))])
            .collect()
    }

    pub fn output_elements<R>(
        &self,
        renderer: &mut R,
        output: &smithay::output::Output,
        with_cursor: bool,
    ) -> Vec<HazelElement<R>>
    where
        R: ImportAll + ImportMem,
        <R as RendererSuper>::TextureId: Send + Clone + 'static,
    {
        let mut elements = Vec::new();
        if with_cursor {
            elements.extend(self.cursor_elements(renderer, output));
        }
        if self.session_lock.is_active() {
            elements.extend(self.locked_elements(renderer, output));
            return elements;
        }
        elements.extend(self.layer_elements(renderer, output, Layer::Overlay));
        elements.extend(self.output_shape_elements(renderer, output, Layer::Overlay));
        elements.extend(self.window_elements(renderer, output, true));
        elements.extend(self.layer_elements(renderer, output, Layer::Top));
        elements.extend(self.output_shape_elements(renderer, output, Layer::Top));
        elements.extend(self.window_elements(renderer, output, false));
        elements.extend(self.layer_elements(renderer, output, Layer::Bottom));
        elements.extend(self.output_shape_elements(renderer, output, Layer::Bottom));
        elements.extend(self.layer_elements(renderer, output, Layer::Background));
        elements.extend(self.output_shape_elements(renderer, output, Layer::Background));
        elements
    }
}

impl Hazel {
    pub fn render_tty_output(
        &mut self,
        node: DrmNode,
        crtc: CrtcHandle,
        handle: LoopHandle<Hazel>,
    ) {
        let tty = match &mut self.backend {
            Backend::Udev(tty) => tty,
            _ => return,
        };

        let Some(device) = tty.output_devices.get_mut(&node) else {
            return;
        };
        let Some(surface) = device.surfaces.get_mut(&crtc) else {
            return;
        };

        let output = surface.output.clone();

        let render_node = device.render_node.unwrap_or(tty.primary_gpu);
        let Ok(mut renderer) = tty.gpu_manager.single_renderer(&render_node) else {
            return;
        };

        let elements = self.compositor.output_elements(&mut renderer, &output, true);

        let clear_color = [0.2, 0.4, 0.8, 1.0];
        let queued = match surface.compositor.render_frame::<_, _>(
            &mut renderer,
            &elements,
            clear_color,
            FrameFlags::DEFAULT,
        ) {
            Ok(result) if !result.is_empty => surface.compositor.queue_frame(()).is_ok(),
            _ => false,
        };

        self.compositor.send_frame_callbacks(&output, Duration::ZERO);

        if queued {
            info!("Flip on {} ({} elements)", output.name(), elements.len());
            return;
        }

        if let Some(refresh) = output.current_mode().map(|m| m.refresh) {
            let interval = Duration::from_micros(1_000_000 / refresh as u64);
            let timer = Timer::from_duration(interval);
            let h = handle.clone();
            let _ = handle.insert_source(timer, move |_, _, state: &mut Hazel| {
                state.render_tty_output(node, crtc, h.clone());
                TimeoutAction::Drop
            });
        }
    }
}
