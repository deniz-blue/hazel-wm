use std::sync::Mutex;

use smithay::{
    output::Output,
    reexports::{
        wayland_protocols::ext::{
            image_capture_source::v1::server::{
                ext_image_capture_source_v1::{self, ExtImageCaptureSourceV1},
                ext_output_image_capture_source_manager_v1::{self, ExtOutputImageCaptureSourceManagerV1},
            },
            image_copy_capture::v1::server::{
                ext_image_copy_capture_cursor_session_v1::{self, ExtImageCopyCaptureCursorSessionV1},
                ext_image_copy_capture_frame_v1::{self, ExtImageCopyCaptureFrameV1, FailureReason},
                ext_image_copy_capture_manager_v1::{self, ExtImageCopyCaptureManagerV1},
                ext_image_copy_capture_session_v1::{self, ExtImageCopyCaptureSessionV1},
            },
        },
        wayland_server::{
            Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, Resource,
            backend::{ClientId, GlobalId},
            protocol::{wl_buffer::WlBuffer, wl_shm},
        },
    },
    utils::{Physical, Size},
};

use crate::core::{
    Hazel,
    capture::target::{CaptureTarget, DmabufCaptureSupport, classify_buffer},
    locking::LockRecovering,
};

const ACCEPTED_FORMATS: [wl_shm::Format; 2] = [wl_shm::Format::Argb8888, wl_shm::Format::Xrgb8888];

pub struct OutputSource(Output);

pub struct SessionData {
    output: Option<Output>,
    size: Mutex<Size<i32, Physical>>,
    has_unfinished_frame: Mutex<bool>,
}

pub struct FrameData {
    session: ExtImageCopyCaptureSessionV1,
    buffer: Mutex<Option<WlBuffer>>,
    captured: Mutex<bool>,
}

pub struct PendingFrame {
    frame: ExtImageCopyCaptureFrameV1,
    session: ExtImageCopyCaptureSessionV1,
    target: CaptureTarget,
}

impl PendingFrame {
    pub fn finish(&self, output: &Output, size: Size<i32, Physical>, now: std::time::Duration) {
        self.frame.transform(output.current_transform().into());
        self.frame.damage(0, 0, size.w, size.h);
        let seconds = now.as_secs();
        self.frame.presentation_time((seconds >> 32) as u32, seconds as u32, now.subsec_nanos());
        self.frame.ready();
        finish_session_frame(&self.session);
    }

    pub fn fail(&self) {
        fail(&self.frame, &self.session, FailureReason::Unknown);
    }

    pub fn target(&self) -> &CaptureTarget {
        &self.target
    }
}

pub struct ImageCaptureState {
    _globals: [GlobalId; 2],
    pending: Vec<PendingFrame>,
}

impl ImageCaptureState {
    pub fn new(display: &DisplayHandle) -> Self {
        Self {
            _globals: [
                display.create_global::<Hazel, ExtOutputImageCaptureSourceManagerV1, ()>(1, ()),
                display.create_global::<Hazel, ExtImageCopyCaptureManagerV1, ()>(1, ()),
            ],
            pending: Vec::new(),
        }
    }

    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    pub fn take_all(&mut self) -> Vec<PendingFrame> {
        std::mem::take(&mut self.pending)
    }

    pub fn take_for_output(&mut self, output: &Output) -> Vec<PendingFrame> {
        let (mine, others) = std::mem::take(&mut self.pending).into_iter().partition(|pending| {
            pending.session.data::<SessionData>().is_some_and(|data| data.output.as_ref() == Some(output))
        });
        self.pending = others;
        mine
    }
}

impl GlobalDispatch<ExtOutputImageCaptureSourceManagerV1, ()> for Hazel {
    fn bind(_: &mut Self, _: &DisplayHandle, _: &Client, resource: New<ExtOutputImageCaptureSourceManagerV1>, _: &(), data_init: &mut DataInit<'_, Self>) {
        data_init.init(resource, ());
    }
}

impl GlobalDispatch<ExtImageCopyCaptureManagerV1, ()> for Hazel {
    fn bind(_: &mut Self, _: &DisplayHandle, _: &Client, resource: New<ExtImageCopyCaptureManagerV1>, _: &(), data_init: &mut DataInit<'_, Self>) {
        data_init.init(resource, ());
    }
}

impl Dispatch<ExtOutputImageCaptureSourceManagerV1, ()> for Hazel {
    fn request(
        _: &mut Self,
        _: &Client,
        _: &ExtOutputImageCaptureSourceManagerV1,
        request: ext_output_image_capture_source_manager_v1::Request,
        _: &(),
        _: &DisplayHandle,
        data_init: &mut DataInit<'_, Self>,
    ) {
        if let ext_output_image_capture_source_manager_v1::Request::CreateSource { source, output } = request {
            match Output::from_resource(&output) {
                Some(output) => {
                    data_init.init(source, Some(OutputSource(output)));
                }
                None => {
                    data_init.init(source, None::<OutputSource>);
                }
            }
        }
    }
}

impl Dispatch<ExtImageCaptureSourceV1, Option<OutputSource>> for Hazel {
    fn request(_: &mut Self, _: &Client, _: &ExtImageCaptureSourceV1, _: ext_image_capture_source_v1::Request, _: &Option<OutputSource>, _: &DisplayHandle, _: &mut DataInit<'_, Self>) {}
}

impl Dispatch<ExtImageCopyCaptureManagerV1, ()> for Hazel {
    fn request(
        state: &mut Self,
        _: &Client,
        _: &ExtImageCopyCaptureManagerV1,
        request: ext_image_copy_capture_manager_v1::Request,
        _: &(),
        _: &DisplayHandle,
        data_init: &mut DataInit<'_, Self>,
    ) {
        use ext_image_copy_capture_manager_v1::Request;
        match request {
            Request::CreateSession { session, source, .. } => {
                let output = source.data::<Option<OutputSource>>().and_then(|source| source.as_ref()).map(|source| source.0.clone());
                let size = output.as_ref().and_then(|output| output.current_mode()).map(|mode| mode.size);
                match (output, size) {
                    (Some(output), Some(size)) if state.backend.supports_screencopy() => {
                        let session = data_init.init(
                            session,
                            SessionData { output: Some(output), size: Mutex::new(size), has_unfinished_frame: Mutex::new(false) },
                        );
                        announce_constraints(&session, size, state.backend.dmabuf_capture_support());
                    }
                    _ => {
                        data_init.init(session, SessionData::stopped()).stopped();
                    }
                }
            }
            Request::CreatePointerCursorSession { session, .. } => {
                data_init.init(session, ());
            }
            _ => {}
        }
    }
}

impl SessionData {
    fn stopped() -> Self {
        Self { output: None, size: Mutex::new(Size::default()), has_unfinished_frame: Mutex::new(true) }
    }
}

impl Dispatch<ExtImageCopyCaptureCursorSessionV1, ()> for Hazel {
    fn request(_: &mut Self, _: &Client, _: &ExtImageCopyCaptureCursorSessionV1, _: ext_image_copy_capture_cursor_session_v1::Request, _: &(), _: &DisplayHandle, _: &mut DataInit<'_, Self>) {}
}

impl Dispatch<ExtImageCopyCaptureSessionV1, SessionData> for Hazel {
    fn request(
        _: &mut Self,
        _: &Client,
        session: &ExtImageCopyCaptureSessionV1,
        request: ext_image_copy_capture_session_v1::Request,
        data: &SessionData,
        _: &DisplayHandle,
        data_init: &mut DataInit<'_, Self>,
    ) {
        if let ext_image_copy_capture_session_v1::Request::CreateFrame { frame } = request {
            let mut busy = data.has_unfinished_frame.lock_recovering();
            data_init.init(
                frame,
                FrameData { session: session.clone(), buffer: Mutex::new(None), captured: Mutex::new(false) },
            );
            if *busy {
                session.post_error(
                    ext_image_copy_capture_session_v1::Error::DuplicateFrame,
                    "the previous frame is not finished",
                );
                return;
            }
            *busy = true;
        }
    }

    fn destroyed(state: &mut Self, _: ClientId, session: &ExtImageCopyCaptureSessionV1, _: &SessionData) {
        state.compositor.smithay.image_capture.pending.retain(|pending| &pending.session != session);
    }
}

impl Dispatch<ExtImageCopyCaptureFrameV1, FrameData> for Hazel {
    fn request(
        state: &mut Self,
        _: &Client,
        frame: &ExtImageCopyCaptureFrameV1,
        request: ext_image_copy_capture_frame_v1::Request,
        data: &FrameData,
        _: &DisplayHandle,
        _: &mut DataInit<'_, Self>,
    ) {
        use ext_image_copy_capture_frame_v1::{Error, Request};
        let mut captured = data.captured.lock_recovering();
        match request {
            Request::AttachBuffer { buffer } => {
                if *captured {
                    frame.post_error(Error::AlreadyCaptured, "the frame was already captured");
                    return;
                }
                *data.buffer.lock_recovering() = Some(buffer);
            }
            Request::Capture => {
                if *captured {
                    frame.post_error(Error::AlreadyCaptured, "the frame was already captured");
                    return;
                }
                let Some(buffer) = data.buffer.lock_recovering().clone() else {
                    frame.post_error(Error::NoBuffer, "capture without an attached buffer");
                    return;
                };
                *captured = true;
                drop(captured);
                state.queue_capture(frame, &data.session, buffer);
            }
            _ => {}
        }
    }

    fn destroyed(state: &mut Self, _: ClientId, frame: &ExtImageCopyCaptureFrameV1, _: &FrameData) {
        state.compositor.smithay.image_capture.pending.retain(|pending| &pending.frame != frame);
    }
}

impl Hazel {
    fn queue_capture(&mut self, frame: &ExtImageCopyCaptureFrameV1, session: &ExtImageCopyCaptureSessionV1, buffer: WlBuffer) {
        let session = session.clone();
        let Some(data) = session.data::<SessionData>() else {
            return;
        };
        let Some(output) = &data.output else {
            fail(frame, &session, FailureReason::Stopped);
            return;
        };
        let current = output.current_mode().map(|mode| mode.size);
        let announced = *data.size.lock_recovering();
        if current != Some(announced) {
            if let Some(size) = current {
                *data.size.lock_recovering() = size;
                announce_constraints(&session, size, self.backend.dmabuf_capture_support());
            }
            fail(frame, &session, FailureReason::BufferConstraints);
            return;
        }
        let support = self.backend.dmabuf_capture_support();
        let Ok(target) = classify_buffer(&buffer, announced, &ACCEPTED_FORMATS, support) else {
            fail(frame, &session, FailureReason::BufferConstraints);
            return;
        };
        self.compositor.smithay.image_capture.pending.push(PendingFrame { frame: frame.clone(), session, target });
    }
}

fn announce_constraints(session: &ExtImageCopyCaptureSessionV1, size: Size<i32, Physical>, dmabuf: Option<&DmabufCaptureSupport>) {
    session.buffer_size(size.w as u32, size.h as u32);
    for format in ACCEPTED_FORMATS {
        session.shm_format(format);
    }
    if let Some(support) = dmabuf {
        session.dmabuf_device(support.device.to_ne_bytes().to_vec());
        for code in support.codes() {
            let modifiers = support
                .modifiers_for(code)
                .into_iter()
                .flat_map(|modifier| u64::from(modifier).to_ne_bytes())
                .collect();
            session.dmabuf_format(code as u32, modifiers);
        }
    }
    session.done();
}

fn fail(frame: &ExtImageCopyCaptureFrameV1, session: &ExtImageCopyCaptureSessionV1, reason: FailureReason) {
    frame.failed(reason);
    finish_session_frame(session);
}

fn finish_session_frame(session: &ExtImageCopyCaptureSessionV1) {
    if let Some(data) = session.data::<SessionData>() {
        *data.has_unfinished_frame.lock_recovering() = false;
    }
}
