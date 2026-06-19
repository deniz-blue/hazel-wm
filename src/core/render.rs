use std::{sync::OnceLock, time::Duration};

use smithay::{
    backend::{
        allocator::Fourcc,
        drm::{DrmNode, compositor::FrameFlags},
        renderer::{
            ImportAll, ImportMem,
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
use tracing::{info, trace};

use crate::{backend::Backend, core::Hazel};

// Element type

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

// Render entry point

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
        let output_geometry = match self.compositor.space.output_geometry(&output) {
            Some(g) => g,
            None => return,
        };

        let render_node = device.render_node.unwrap_or(tty.primary_gpu);
        let Ok(mut renderer) = tty.gpu_manager.single_renderer(&render_node) else {
            return;
        };

        let scale = Scale::from(output.current_scale().fractional_scale());

        // Cursor
        // render_output() reverses the element slice and draws back-to-front,
        // so front-most elements must come FIRST.
        let cursor_buf = cursor_render_buffer();

        let mut elements: Vec<HazelElement<_>> = Vec::new();

        for seat in self.compositor.seats.values() {
            let Some(pointer) = seat.get_pointer() else {
                continue;
            };
            let Some(state) = self.compositor.pointers.get(&pointer) else {
                continue;
            };

            let cursor_global = pointer.current_location();
            if !output_geometry.to_f64().contains(cursor_global) {
                continue;
            }

            let cursor_phys = (cursor_global - output_geometry.loc.to_f64())
                .to_physical(scale)
                .to_i32_round();

            match &state.cursor_status {
                CursorImageStatus::Surface(surf) if surf.alive() => {
                    // Offset by the cursor hotspot so the pointer tip
                    // matches where the user clicked.
                    let hotspot = smithay::wayland::compositor::with_states(surf, |states| {
                        states
                            .data_map
                            .get::<smithay::input::pointer::CursorImageSurfaceData>()
                            .map(|d| d.lock().unwrap().hotspot)
                    })
                    .unwrap_or_default();

                    let pos = cursor_phys - hotspot.to_f64().to_physical(scale).to_i32_round();
                    let tree = SurfaceTree::from_surface(surf);
                    let els: Vec<HazelElement<_>> = tree
                        .render_elements(&mut renderer, pos, scale, 1.0)
                        .into_iter()
                        .map(HazelElement::Space)
                        .collect();
                    elements.extend(els);
                    trace!("cursor: surface");
                }
                _ => {
                    trace!("cursor: fallback");
                    let hotspot = CURSOR_SIZE / 2;
                    let pos = (
                        (cursor_phys.x - hotspot) as f64,
                        (cursor_phys.y - hotspot) as f64,
                    );
                    let el = MemoryRenderBufferRenderElement::from_buffer(
                        &mut renderer,
                        pos,
                        cursor_buf,
                        None,
                        None,
                        Some(Size::from((CURSOR_SIZE, CURSOR_SIZE))),
                        Kind::Cursor,
                    );
                    if let Ok(el) = el {
                        elements.push(HazelElement::Memory(el));
                    }
                }
            }
        }

        let mut bottom_layer_elements: Vec<HazelElement<_>> = Vec::new();
        let mut top_layer_elements: Vec<HazelElement<_>> = Vec::new();

        let layer_map = layer_map_for_output(&output);
        for smithay_layer in &[Layer::Background, Layer::Bottom] {
            for layer_surface in layer_map.layers_on(*smithay_layer) {
                if !layer_surface.alive() || !layer_surface.wl_surface().alive() {
                    continue;
                }
                let Some(geo) = layer_map.layer_geometry(layer_surface) else {
                    continue;
                };
                let pos = geo.loc.to_f64().to_physical(scale).to_i32_round();
                let tree = SurfaceTree::from_surface(layer_surface.wl_surface());
                let els: Vec<HazelElement<_>> = tree
                    .render_elements(&mut renderer, pos, scale, 1.0)
                    .into_iter()
                    .map(HazelElement::Space)
                    .collect();
                bottom_layer_elements.extend(els);
            }
        }
        for smithay_layer in &[Layer::Top, Layer::Overlay] {
            for layer_surface in layer_map.layers_on(*smithay_layer) {
                if !layer_surface.alive() || !layer_surface.wl_surface().alive() {
                    continue;
                }
                let Some(geo) = layer_map.layer_geometry(layer_surface) else {
                    continue;
                };
                let pos = geo.loc.to_f64().to_physical(scale).to_i32_round();
                let tree = SurfaceTree::from_surface(layer_surface.wl_surface());
                let els: Vec<HazelElement<_>> = tree
                    .render_elements(&mut renderer, pos, scale, 1.0)
                    .into_iter()
                    .map(HazelElement::Space)
                    .collect();
                top_layer_elements.extend(els);
            }
        }
        drop(layer_map);

        use smithay::desktop::space::SpaceRenderElements;
        let space_elements: Vec<HazelElement<_>> = self
            .compositor
            .space
            .render_elements_for_output(&mut renderer, &output, 1.0)
            .unwrap_or_default()
            .into_iter()
            .flat_map(|se| match se {
                SpaceRenderElements::Surface(el) => vec![HazelElement::Space(el)],
                SpaceRenderElements::Element(w) => {
                    vec![HazelElement::from(w)]
                }
                _ => vec![],
            })
            .collect();

        elements.extend(top_layer_elements);
        elements.extend(space_elements);
        elements.extend(bottom_layer_elements);

        // Cursor remains first in the vec (front-most).

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

        // Nothing changed — retry after one refresh interval.
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
