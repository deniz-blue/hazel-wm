use smithay::{
    backend::{
        allocator::gbm::{GbmAllocator, GbmDevice},
        drm::{DrmDevice, DrmDeviceFd, DrmNode},
    },
    reexports::calloop::RegistrationToken,
};

pub struct OutputDevice {
    pub token: RegistrationToken,
    pub render_node: Option<DrmNode>,
    pub drm: DrmDevice,
    pub gbm: GbmDevice<DrmDeviceFd>,
    pub allocator: GbmAllocator<DrmDeviceFd>,
}
