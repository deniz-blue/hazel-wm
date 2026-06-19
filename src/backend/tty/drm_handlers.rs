use std::time::Duration;

use smithay::{
    backend::drm::{
        DrmEventMetadata, DrmNode,
        compositor::FrameFlags,
    },
    desktop::{Space, Window},
    reexports::{
        calloop::{LoopHandle, timer::{TimeoutAction, Timer}},
        drm::control::crtc::{Handle as CrtcHandle},
    },
};
use tracing::{error, info};

use crate::{
    backend::Backend,
    core::Hazel,
};

use super::HazelTtyBackend;

impl HazelTtyBackend {
    /// Called from the DRM VBlank notifier. Marks the previous frame as submitted.
    pub fn on_vblank(
        &mut self,
        _space: &mut Space<Window>,
        node: DrmNode,
        crtc: CrtcHandle,
        _meta: DrmEventMetadata,
    ) {
        let Some(device) = self.output_devices.get_mut(&node) else {
            return;
        };
        let Some(surface) = device.surfaces.get_mut(&crtc) else {
            return;
        };
        let _ = surface.compositor.frame_submitted();
    }

    /// Render the current space contents for a given output and queue a page flip.
    /// Always schedules a retry — via VBlank if a flip was queued, or via timer otherwise.
    pub fn render_and_queue(
        &mut self,
        space: &mut Space<Window>,
        node: DrmNode,
        crtc: CrtcHandle,
        handle: LoopHandle<Hazel>,
    ) {
        let handle_clone = handle.clone();

        let refresh_interval = {
            let Some(device) = self.output_devices.get_mut(&node) else {
                return;
            };
            let Some(surface) = device.surfaces.get_mut(&crtc) else {
                return;
            };

            let output = surface.output.clone();

            let render_node = device.render_node.unwrap_or(self.primary_gpu);
            let mut renderer = match self.gpu_manager.single_renderer(&render_node) {
                Ok(r) => r,
                Err(e) => {
                    error!("Failed to get renderer for {}: {e}", output.name());
                    return;
                }
            };

            let elements = match space.render_elements_for_output(&mut renderer, &output, 1.0) {
                Ok(e) => e,
                Err(_) => {
                    // Output not mapped yet.
                    return;
                }
            };

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

            // Send frame callbacks regardless of whether we queued a flip.
            for window in space.elements() {
                window.send_frame(
                    &output,
                    Duration::ZERO,
                    Some(Duration::ZERO),
                    |_, _| Some(output.clone()),
                );
            }

            if queued {
                info!("Flip on {} ({} elements)", output.name(), elements.len());
                // VBlank will call on_vblank → frame_submitted, and the DRM notifier
                // callback will schedule the next render timer.
                return;
            }

            // Nothing changed — retry after one refresh interval.
            output.current_mode().map(|m| m.refresh)
        };

        // Schedule a retry timer if we didn't queue a flip (no VBlank coming).
        if let Some(refresh) = refresh_interval {
            let interval = Duration::from_micros(1_000_000 / refresh as u64);
            let timer = Timer::from_duration(interval);
            let _ = handle.insert_source(timer, move |_, _, state: &mut Hazel| {
                let space = &mut state.compositor.space;
                let tty = match &mut state.backend {
                    Backend::Udev(tty) => tty,
                    _ => return TimeoutAction::Drop,
                };
                tty.render_and_queue(space, node, crtc, handle_clone.clone());
                TimeoutAction::Drop
            });
        }
    }

    /// Kick off the first frame for a newly created surface.
    pub fn kickstart_surface(
        &mut self,
        space: &mut Space<Window>,
        node: DrmNode,
        crtc: CrtcHandle,
        handle: LoopHandle<Hazel>,
    ) {
        self.render_and_queue(space, node, crtc, handle.clone());
    }
}
