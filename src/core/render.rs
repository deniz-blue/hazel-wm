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
                surface::WaylandSurfaceRenderElement,
            },
        },
    },
    desktop::{layer_map_for_output, space::SurfaceTree},
    input::pointer::CursorImageStatus,
    reexports::{
        calloop::{
            LoopHandle,
            timer::{TimeoutAction, Timer},
        },
        drm::control::crtc::Handle as CrtcHandle,
    },
    utils::{IsAlive, Scale, Size, Transform},
    wayland::shell::wlr_layer::Layer,
};
use tracing::info;

use crate::{
    backend::Backend,
    core::{Hazel, compositor::HazelCompositor},
};

smithay::backend::renderer::element::render_elements! {
    pub HazelElement<R> where R: ImportAll + ImportMem;
    Space=WaylandSurfaceRenderElement<R>,
    Wrapped=smithay::backend::renderer::element::Wrap<WaylandSurfaceRenderElement<R>>,
    Memory=MemoryRenderBufferRenderElement<R>,
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
                            .map(|d| d.lock().unwrap().hotspot)
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
    ) -> Vec<HazelElement<R>>
    where
        R: ImportAll + ImportMem,
        <R as RendererSuper>::TextureId: Clone + 'static,
    {
        let output_geometry = match self.space.output_geometry(output) {
            Some(g) => g,
            None => return Vec::new(),
        };

        let output_scale = Scale::from(output.current_scale().fractional_scale());

        let mut elements = Vec::new();

        for window in self.space.elements() {
            let Some(window_geo) = self.space.element_geometry(window) else {
                continue;
            };
            let pos = (window_geo.loc - output_geometry.loc)
                .to_f64()
                .to_physical(output_scale)
                .to_i32_round();
            elements.extend(window.render_elements::<HazelElement<_>>(
                renderer,
                pos,
                output_scale,
                1.0,
            ));
        }

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

        let mut elements: Vec<HazelElement<_>> = Vec::new();

        elements.extend(self.compositor.cursor_elements(&mut renderer, &output));

        elements.extend(
            self.compositor
                .layer_elements(&mut renderer, &output, Layer::Overlay),
        );

        elements.extend(
            self.compositor
                .layer_elements(&mut renderer, &output, Layer::Top),
        );

        elements.extend(self.compositor.window_elements(&mut renderer, &output));

        elements.extend(
            self.compositor
                .layer_elements(&mut renderer, &output, Layer::Bottom),
        );

        elements.extend(
            self.compositor
                .layer_elements(&mut renderer, &output, Layer::Background),
        );

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

        for window in self.compositor.space.elements() {
            window.send_frame(&output, Duration::ZERO, Some(Duration::ZERO), |_, _| {
                Some(output.clone())
            });
        }

        let layer_map = layer_map_for_output(&output);
        for layer_surface in layer_map.layers() {
            if layer_surface.alive() {
                layer_surface.send_frame(&output, Duration::ZERO, Some(Duration::ZERO), |_, _| {
                    Some(output.clone())
                });
            }
        }
        drop(layer_map);

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
