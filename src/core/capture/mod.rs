use smithay::{
    output::Output,
    utils::{Physical, Rectangle, Size},
};

use crate::core::{
    Hazel,
    handlers::{image_copy_capture::PendingFrame, screencopy::PendingCopy},
};
use target::CaptureTarget;

pub mod shm;
pub mod target;

enum Requester {
    Screencopy(PendingCopy),
    ImageCopy(PendingFrame),
}

pub struct CaptureRequest {
    pub target: CaptureTarget,
    pub region: Rectangle<i32, Physical>,
    requester: Requester,
}

impl Hazel {
    pub fn has_pending_captures(&self) -> bool {
        self.compositor.smithay.screencopy.has_pending() || self.compositor.smithay.image_capture.has_pending()
    }

    pub fn take_capture_requests(&mut self, output: &Output) -> Vec<CaptureRequest> {
        let size = output.current_mode().map(|mode| mode.size).unwrap_or_default();
        let screencopies = self.compositor.smithay.screencopy.take_for_output(output).into_iter().map(|request| CaptureRequest {
            target: request.target,
            region: request.pending.region(),
            requester: Requester::Screencopy(request.pending),
        });
        let image_copies = self.compositor.smithay.image_capture.take_for_output(output).into_iter().map(move |pending| CaptureRequest {
            target: pending.target().clone(),
            region: Rectangle::from_size(size),
            requester: Requester::ImageCopy(pending),
        });
        screencopies.chain(image_copies).collect()
    }

    pub fn finish_capture(&mut self, request: CaptureRequest, output: &Output, size: Size<i32, Physical>) {
        let now = std::time::Duration::from(self.compositor.presentation_clock.now());
        match request.requester {
            Requester::Screencopy(pending) => pending.finish(now),
            Requester::ImageCopy(pending) => pending.finish(output, size, now),
        }
    }

    pub fn fail_capture(&mut self, request: CaptureRequest) {
        match request.requester {
            Requester::Screencopy(pending) => pending.fail(),
            Requester::ImageCopy(pending) => pending.fail(),
        }
    }

    pub fn fail_pending_captures(&mut self) {
        let screencopies = self.compositor.smithay.screencopy.take_all();
        let image_copies = self.compositor.smithay.image_capture.take_all();
        screencopies.into_iter().for_each(|request| request.pending.fail());
        image_copies.into_iter().for_each(|pending| pending.fail());
    }
}
