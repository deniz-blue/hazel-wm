use std::collections::HashMap;

use smithay::{
    backend::{
        allocator::gbm::{GbmAllocator, GbmDevice},
        drm::{
            DrmDevice, DrmDeviceFd, DrmNode, compositor::DrmCompositor,
            exporter::gbm::GbmFramebufferExporter,
        },
    },
    output::Output,
    reexports::{calloop::RegistrationToken, drm::control::connector, drm::control::crtc},
};
use smithay_drm_extras::drm_scanner::DrmScanner;

pub type HazelDrmCompositor =
    DrmCompositor<GbmAllocator<DrmDeviceFd>, GbmFramebufferExporter<DrmDeviceFd>, (), DrmDeviceFd>;

#[derive(Debug)]
pub struct Surface {
    pub compositor: HazelDrmCompositor,
    pub connector: connector::Handle,
    pub output: Output,
}

#[derive(Debug)]
pub struct OutputDevice {
    pub token: RegistrationToken,
    pub render_node: Option<DrmNode>,
    pub drm: DrmDevice,
    pub gbm: GbmDevice<DrmDeviceFd>,
    pub allocator: GbmAllocator<DrmDeviceFd>,
    pub drm_scanner: DrmScanner,
    pub surfaces: HashMap<crtc::Handle, Surface>,
}
