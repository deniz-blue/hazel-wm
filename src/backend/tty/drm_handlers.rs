use smithay::{
    backend::drm::{DrmEventMetadata, DrmNode},
    reexports::drm::control::crtc::Handle as CrtcHandle,
};

use super::HazelTtyBackend;

impl HazelTtyBackend {
    /// Called from the DRM VBlank notifier. Marks the previous frame as submitted.
    pub fn on_vblank(&mut self, node: DrmNode, crtc: CrtcHandle, _meta: DrmEventMetadata) {
        let Some(device) = self.output_devices.get_mut(&node) else {
            return;
        };
        let Some(surface) = device.surfaces.get_mut(&crtc) else {
            return;
        };
        let _ = surface.compositor.frame_submitted();
    }
}
