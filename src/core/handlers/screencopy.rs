use std::sync::atomic::{AtomicBool, Ordering};

use smithay::{
    output::Output,
    reexports::{
        wayland_protocols_wlr::screencopy::v1::server::{
            zwlr_screencopy_frame_v1::{self, Flags, ZwlrScreencopyFrameV1},
            zwlr_screencopy_manager_v1::{self, ZwlrScreencopyManagerV1},
        },
        wayland_server::{
            Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, Resource,
            backend::GlobalId,
            protocol::wl_shm,
        },
    },
    utils::{Physical, Rectangle, Size},
};

use crate::core::{
    Hazel,
    capture::{
        shm::BYTES_PER_PIXEL,
        target::{CaptureTarget, DmabufCaptureSupport, classify_buffer},
    },
};

const VERSION: u32 = 3;
const FORMAT: wl_shm::Format = wl_shm::Format::Argb8888;

pub struct FrameRequest {
    output: Option<Output>,
    region: Rectangle<i32, Physical>,
    used: AtomicBool,
}

pub struct PendingCopy {
    frame: ZwlrScreencopyFrameV1,
    output: Output,
    region: Rectangle<i32, Physical>,
    with_damage: bool,
}

pub struct ScreencopyRequest {
    pub pending: PendingCopy,
    pub target: CaptureTarget,
}

pub struct ScreencopyState {
    _global: GlobalId,
    pending: Vec<ScreencopyRequest>,
}

impl ScreencopyState {
    pub fn new(display: &DisplayHandle) -> Self {
        Self {
            _global: display.create_global::<Hazel, ZwlrScreencopyManagerV1, ()>(VERSION, ()),
            pending: Vec::new(),
        }
    }

    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    pub fn take_all(&mut self) -> Vec<ScreencopyRequest> {
        std::mem::take(&mut self.pending)
    }

    pub fn take_for_output(&mut self, output: &Output) -> Vec<ScreencopyRequest> {
        let (mine, others) = std::mem::take(&mut self.pending)
            .into_iter()
            .partition(|request| &request.pending.output == output);
        self.pending = others;
        mine
    }
}

impl PendingCopy {
    pub fn region(&self) -> Rectangle<i32, Physical> {
        self.region
    }

    pub fn finish(&self, now: std::time::Duration) {
        self.frame.flags(Flags::empty());
        if self.with_damage {
            self.frame.damage(0, 0, self.region.size.w as u32, self.region.size.h as u32);
        }
        let seconds = now.as_secs();
        self.frame.ready((seconds >> 32) as u32, seconds as u32, now.subsec_nanos());
    }

    pub fn fail(&self) {
        self.frame.failed();
    }
}

impl GlobalDispatch<ZwlrScreencopyManagerV1, ()> for Hazel {
    fn bind(
        _: &mut Self,
        _: &DisplayHandle,
        _: &Client,
        resource: New<ZwlrScreencopyManagerV1>,
        _: &(),
        data_init: &mut DataInit<'_, Self>,
    ) {
        data_init.init(resource, ());
    }
}

impl Dispatch<ZwlrScreencopyManagerV1, ()> for Hazel {
    fn request(
        state: &mut Self,
        _: &Client,
        _: &ZwlrScreencopyManagerV1,
        request: zwlr_screencopy_manager_v1::Request,
        _: &(),
        _: &DisplayHandle,
        data_init: &mut DataInit<'_, Self>,
    ) {
        use zwlr_screencopy_manager_v1::Request;
        let (frame, output, logical_region) = match request {
            Request::CaptureOutput { frame, output, .. } => (frame, output, None),
            Request::CaptureOutputRegion { frame, output, x, y, width, height, .. } => {
                (frame, output, Some((x, y, width, height)))
            }
            _ => return,
        };
        let Some(output) = Output::from_resource(&output) else {
            data_init.init(frame, unavailable()).failed();
            return;
        };
        let size = output.current_mode().map(|mode| mode.size);
        let region = match (size, logical_region) {
            (Some(size), None) => Some(Rectangle::from_size(size)),
            (Some(size), Some(region)) => physical_region(&output, region, size),
            (None, _) => None,
        };
        match region {
            Some(region) if state.backend.supports_screencopy() => {
                let frame = data_init.init(
                    frame,
                    FrameRequest { output: Some(output), region, used: AtomicBool::new(false) },
                );
                announce_buffer(&frame, region.size, state.backend.dmabuf_capture_support());
            }
            _ => data_init.init(frame, unavailable()).failed(),
        }
    }
}

impl Dispatch<ZwlrScreencopyFrameV1, FrameRequest> for Hazel {
    fn request(
        state: &mut Self,
        _: &Client,
        frame: &ZwlrScreencopyFrameV1,
        request: zwlr_screencopy_frame_v1::Request,
        data: &FrameRequest,
        _: &DisplayHandle,
        _: &mut DataInit<'_, Self>,
    ) {
        use zwlr_screencopy_frame_v1::Request;
        let (buffer, with_damage) = match request {
            Request::Copy { buffer } => (buffer, false),
            Request::CopyWithDamage { buffer } => (buffer, true),
            _ => return,
        };
        if data.used.swap(true, Ordering::SeqCst) {
            frame.post_error(
                zwlr_screencopy_frame_v1::Error::AlreadyUsed,
                "the frame was already copied",
            );
            return;
        }
        let support = state.backend.dmabuf_capture_support();
        let target = match classify_buffer(&buffer, data.region.size, &[FORMAT], support) {
            Ok(target) => target,
            Err(reason) => {
                frame.post_error(zwlr_screencopy_frame_v1::Error::InvalidBuffer, reason);
                return;
            }
        };
        let Some(output) = data.output.clone() else {
            frame.failed();
            return;
        };
        let pending = PendingCopy { frame: frame.clone(), output, region: data.region, with_damage };
        state.compositor.smithay.screencopy.pending.push(ScreencopyRequest { pending, target });
    }

    fn destroyed(
        state: &mut Self,
        _: smithay::reexports::wayland_server::backend::ClientId,
        frame: &ZwlrScreencopyFrameV1,
        _: &FrameRequest,
    ) {
        state
            .compositor
            .smithay
            .screencopy
            .pending
            .retain(|request| &request.pending.frame != frame);
    }
}

fn announce_buffer(frame: &ZwlrScreencopyFrameV1, size: Size<i32, Physical>, dmabuf: Option<&DmabufCaptureSupport>) {
    frame.buffer(
        FORMAT,
        size.w as u32,
        size.h as u32,
        size.w as u32 * BYTES_PER_PIXEL as u32,
    );
    if frame.version() >= 3 {
        for code in dmabuf.map(DmabufCaptureSupport::codes).unwrap_or_default() {
            frame.linux_dmabuf(code as u32, size.w as u32, size.h as u32);
        }
        frame.buffer_done();
    }
}

fn unavailable() -> FrameRequest {
    FrameRequest {
        output: None,
        region: Rectangle::default(),
        used: AtomicBool::new(true),
    }
}

fn physical_region(
    output: &Output,
    (x, y, width, height): (i32, i32, i32, i32),
    size: Size<i32, Physical>,
) -> Option<Rectangle<i32, Physical>> {
    let scale = output.current_scale().fractional_scale();
    let to_physical = |value: i32| (f64::from(value) * scale).round() as i32;
    let wanted = Rectangle::<i32, Physical>::new(
        (to_physical(x), to_physical(y)).into(),
        (to_physical(width), to_physical(height)).into(),
    );
    let transform = output.current_transform();
    let wanted = transform.invert().transform_rect_in(wanted, &transform.transform_size(size));
    let clipped = wanted.intersection(Rectangle::from_size(size))?;
    (width > 0 && height > 0 && !clipped.is_empty()).then_some(clipped)
}

#[cfg(test)]
mod tests {
    use smithay::{
        output::{Output, PhysicalProperties, Scale, Subpixel},
        utils::Transform,
    };

    use super::*;

    fn output(scale: f64, transform: Transform) -> Output {
        let output = Output::new("test".into(), PhysicalProperties {
                size: (0, 0).into(),
                subpixel: Subpixel::Unknown,
                make: String::new(),
                model: String::new(),
                serial_number: String::new(),
            });
        output.change_current_state(None, Some(transform), Some(Scale::Fractional(scale)), None);
        output
    }

    fn rect(x: i32, y: i32, w: i32, h: i32) -> Rectangle<i32, Physical> {
        Rectangle::new((x, y).into(), (w, h).into())
    }

    #[test]
    fn region_is_scaled_to_physical_pixels() {
        let output = output(2.0, Transform::Normal);
        let region = physical_region(&output, (10, 20, 30, 40), (800, 600).into());
        assert_eq!(region, Some(rect(20, 40, 60, 80)));
    }

    #[test]
    fn region_follows_the_output_transform() {
        let output = output(1.0, Transform::Flipped180);
        let region = physical_region(&output, (0, 0, 240, 40), (800, 600).into());
        assert_eq!(region, Some(rect(0, 560, 240, 40)));
    }

    #[test]
    fn region_keeps_its_place_on_a_rotated_output() {
        let region = |transform| physical_region(&output(1.0, transform), (10, 20, 30, 40), (800, 600).into());
        assert_eq!(region(Transform::_90), Some(rect(20, 560, 40, 30)));
        assert_eq!(region(Transform::_270), Some(rect(740, 10, 40, 30)));
    }

    #[test]
    fn region_is_clipped_to_the_output() {
        let output = output(1.0, Transform::Normal);
        let region = physical_region(&output, (700, 500, 300, 300), (800, 600).into());
        assert_eq!(region, Some(rect(700, 500, 100, 100)));
    }

    #[test]
    fn region_outside_the_output_is_rejected() {
        let output = output(1.0, Transform::Normal);
        assert_eq!(physical_region(&output, (900, 0, 10, 10), (800, 600).into()), None);
        assert_eq!(physical_region(&output, (0, 0, 0, 10), (800, 600).into()), None);
    }
}
