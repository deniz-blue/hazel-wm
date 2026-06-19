use smithay::{
    backend::drm::{DrmEvent, DrmEventMetadata, DrmNode},
    reexports::drm::control::crtc::Handle as CrtcHandle,
};
use tracing::{info, warn};

use crate::backend::tty::HazelTtyBackend;

impl HazelTtyBackend {
    pub fn on_drm_event(
        &mut self,
        event: DrmEvent,
        node: DrmNode,
        meta: &mut Option<DrmEventMetadata>,
    ) {
        match event {
            DrmEvent::VBlank(crtc) => {
                let meta = meta.expect("VBlank events must have metadata");
                self.on_vblank(node, crtc, meta);
            }
            DrmEvent::Error(error) => warn!("DRM error: {error}"),
            _ => {}
        }
    }

    pub fn on_vblank(&mut self, node: DrmNode, crtc: CrtcHandle, meta: DrmEventMetadata) {
        // Handle VBlank event for the given node and CRTC
        // This is where you would typically trigger a redraw or update the display
        info!("Received VBlank event for node {node:?} on CRTC {crtc:?}");
    }
}
