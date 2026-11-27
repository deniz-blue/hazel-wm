use smithay::{
    input::pointer::{
        AxisFrame, ButtonEvent, Focus, GestureHoldBeginEvent, GestureHoldEndEvent,
        GesturePinchBeginEvent, GesturePinchEndEvent, GesturePinchUpdateEvent,
        GestureSwipeBeginEvent, GestureSwipeEndEvent, GestureSwipeUpdateEvent, GrabStartData,
        MotionEvent, PointerGrab, PointerHandle, PointerInnerHandle, RelativeMotionEvent,
    },
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::{Logical, Point, SERIAL_COUNTER},
};

use crate::core::Hazel;

pub struct ExclusivePointerGrab {
    start_data: GrabStartData<Hazel>,
}

impl ExclusivePointerGrab {
    pub fn new(pointer: &PointerHandle<Hazel>) -> Self {
        Self {
            start_data: GrabStartData {
                focus: None,
                button: 0,
                location: pointer.current_location(),
            },
        }
    }
}

type Focused = Option<(WlSurface, Point<f64, Logical>)>;

impl PointerGrab<Hazel> for ExclusivePointerGrab {
    fn motion(
        &mut self,
        data: &mut Hazel,
        handle: &mut PointerInnerHandle<'_, Hazel>,
        _focus: Focused,
        event: &MotionEvent,
    ) {
        handle.motion(data, None, event);
    }

    fn relative_motion(
        &mut self,
        data: &mut Hazel,
        handle: &mut PointerInnerHandle<'_, Hazel>,
        _focus: Focused,
        event: &RelativeMotionEvent,
    ) {
        handle.relative_motion(data, None, event);
    }

    fn button(
        &mut self,
        data: &mut Hazel,
        handle: &mut PointerInnerHandle<'_, Hazel>,
        event: &ButtonEvent,
    ) {
        handle.button(data, event);
    }

    fn axis(&mut self, data: &mut Hazel, handle: &mut PointerInnerHandle<'_, Hazel>, details: AxisFrame) {
        handle.axis(data, details);
    }

    fn frame(&mut self, data: &mut Hazel, handle: &mut PointerInnerHandle<'_, Hazel>) {
        handle.frame(data);
    }

    fn gesture_swipe_begin(&mut self, data: &mut Hazel, handle: &mut PointerInnerHandle<'_, Hazel>, event: &GestureSwipeBeginEvent) {
        handle.gesture_swipe_begin(data, event);
    }

    fn gesture_swipe_update(&mut self, data: &mut Hazel, handle: &mut PointerInnerHandle<'_, Hazel>, event: &GestureSwipeUpdateEvent) {
        handle.gesture_swipe_update(data, event);
    }

    fn gesture_swipe_end(&mut self, data: &mut Hazel, handle: &mut PointerInnerHandle<'_, Hazel>, event: &GestureSwipeEndEvent) {
        handle.gesture_swipe_end(data, event);
    }

    fn gesture_pinch_begin(&mut self, data: &mut Hazel, handle: &mut PointerInnerHandle<'_, Hazel>, event: &GesturePinchBeginEvent) {
        handle.gesture_pinch_begin(data, event);
    }

    fn gesture_pinch_update(&mut self, data: &mut Hazel, handle: &mut PointerInnerHandle<'_, Hazel>, event: &GesturePinchUpdateEvent) {
        handle.gesture_pinch_update(data, event);
    }

    fn gesture_pinch_end(&mut self, data: &mut Hazel, handle: &mut PointerInnerHandle<'_, Hazel>, event: &GesturePinchEndEvent) {
        handle.gesture_pinch_end(data, event);
    }

    fn gesture_hold_begin(&mut self, data: &mut Hazel, handle: &mut PointerInnerHandle<'_, Hazel>, event: &GestureHoldBeginEvent) {
        handle.gesture_hold_begin(data, event);
    }

    fn gesture_hold_end(&mut self, data: &mut Hazel, handle: &mut PointerInnerHandle<'_, Hazel>, event: &GestureHoldEndEvent) {
        handle.gesture_hold_end(data, event);
    }

    fn start_data(&self) -> &GrabStartData<Hazel> {
        &self.start_data
    }

    fn unset(&mut self, _data: &mut Hazel) {}
}

pub fn holds_exclusive_grab(pointer: &PointerHandle<Hazel>) -> bool {
    pointer
        .with_grab(|_, grab| grab.downcast_ref::<ExclusivePointerGrab>().is_some())
        .unwrap_or(false)
}

impl Hazel {
    pub fn grab_pointer_exclusively(&mut self, pointer: &PointerHandle<Hazel>) {
        pointer.set_grab(
            self,
            ExclusivePointerGrab::new(pointer),
            SERIAL_COUNTER.next_serial(),
            Focus::Clear,
        );
    }

    pub fn release_pointer_grab(&mut self, pointer: &PointerHandle<Hazel>) {
        if !holds_exclusive_grab(pointer) {
            return;
        }
        let serial = SERIAL_COUNTER.next_serial();
        pointer.unset_grab(self, serial, 0);
        let location = pointer.current_location();
        let under = self.compositor.input_under(location);
        pointer.motion(self, under, &MotionEvent { location, serial, time: 0 });
        pointer.frame(self);
    }
}
