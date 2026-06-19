use std::time::Duration;

use smithay::{
    backend::{
        drm::{DrmNode, compositor::FrameFlags},
        renderer::{
            element::{
                AsRenderElements, Kind,
                solid::{SolidColorBuffer, SolidColorRenderElement},
                surface::WaylandSurfaceRenderElement,
            },
            ImportAll,
        },
    },
    desktop::space::SurfaceTree,
    input::pointer::CursorImageStatus,
    reexports::{
        calloop::{LoopHandle, timer::{TimeoutAction, Timer}},
        drm::control::crtc::Handle as CrtcHandle,
    },
    utils::{IsAlive, Scale},
};
use tracing::{info, trace};

use crate::{backend::Backend, core::Hazel};

// ── Unified element type ───────────────────────────────────────────────────

smithay::backend::renderer::element::render_elements! {
    pub HazelElement<R> where R: ImportAll;
    Space=WaylandSurfaceRenderElement<R>,
    Wrapped=smithay::backend::renderer::element::Wrap<WaylandSurfaceRenderElement<R>>,
    Solid=SolidColorRenderElement,
}

// ── Render entry point ─────────────────────────────────────────────────────

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

        // ── Cursor ─────────────────────────────────────────────────────────
        // render_output() reverses the element slice and draws back-to-front,
        // so front-most elements must come FIRST.
        let solid_cursor = SolidColorBuffer::new((16, 16), [1.0, 1.0, 1.0, 1.0]);

        let mut elements: Vec<HazelElement<_>> = Vec::new();

        for (_, seat) in &self.compositor.seats {
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
                    let tree = SurfaceTree::from_surface(surf);
                    let els: Vec<HazelElement<_>> = tree
                        .render_elements(&mut renderer, cursor_phys, scale, 1.0)
                        .into_iter()
                        .map(HazelElement::Space)
                        .collect();
                    elements.extend(els);
                    trace!("cursor: surface");
                }
                _ => {
                    trace!("cursor: fallback cross");
                    let el = HazelElement::Solid(
                        SolidColorRenderElement::from_buffer(
                            &solid_cursor,
                            cursor_phys,
                            1.0,
                            1.0,
                            Kind::Cursor,
                        ),
                    );
                    elements.push(el);
                }
            }
        }

        // ── Space elements (windows + layer surfaces) ──────────────────────
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

        // Cursor on top (front), then space elements behind.
        elements.extend(space_elements);

        // ── Render & queue ─────────────────────────────────────────────────
        let clear_color = [0.2, 0.4, 0.8, 1.0];
        let queued = match surface.compositor.render_frame::<_, _>(
            &mut renderer,
            &elements,
            clear_color,
            FrameFlags::DEFAULT,
        ) {
            Ok(result) if !result.is_empty => {
                surface.compositor.queue_frame(()).is_ok()
            }
            _ => false,
        };

        // Frame callbacks
        for window in self.compositor.space.elements() {
            window.send_frame(&output, Duration::ZERO, Some(Duration::ZERO), |_, _| {
                Some(output.clone())
            });
        }

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
