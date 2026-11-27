use smithay::{
    backend::input::{
        Device, Event, GestureBeginEvent, GestureEndEvent, GesturePinchUpdateEvent, GestureSwipeUpdateEvent,
    },
    input::pointer::{
        GestureHoldBeginEvent, GestureHoldEndEvent, GesturePinchBeginEvent, GesturePinchEndEvent,
        GesturePinchUpdateEvent as PinchUpdate, GestureSwipeBeginEvent, GestureSwipeEndEvent,
        GestureSwipeUpdateEvent as SwipeUpdate, PointerHandle,
    },
    utils::{Logical, Point, SERIAL_COUNTER},
};

use crate::{core::Hazel, lua::api::wm_input_gestures::PointerGestureEvent};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GestureKind {
    Swipe,
    Pinch,
    Hold,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GesturePhase {
    Begin,
    Update,
    End,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gesture {
    pub kind: GestureKind,
    pub phase: GesturePhase,
    pub fingers: u32,
    pub delta: Point<f64, Logical>,
    pub scale: f64,
    pub rotation: f64,
    pub cancelled: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActiveGesture {
    pub kind: GestureKind,
    pub fingers: u32,
    pub forwarded: bool,
}

impl Gesture {
    pub fn new(kind: GestureKind, phase: GesturePhase) -> Self {
        Self {
            kind,
            phase,
            fingers: 0,
            delta: (0.0, 0.0).into(),
            scale: 1.0,
            rotation: 0.0,
            cancelled: false,
        }
    }

    pub fn check_order(&self, active: Option<&ActiveGesture>) -> Result<(), String> {
        match (self.phase, active) {
            (GesturePhase::Begin, Some(_)) => Err("a gesture is already in progress".to_owned()),
            (GesturePhase::Begin, None) => Ok(()),
            (_, None) => Err("no gesture is in progress".to_owned()),
            (_, Some(active)) if active.kind != self.kind => {
                Err(format!("a {:?} gesture is in progress, not a {:?} one", active.kind, self.kind).to_lowercase())
            }
            (GesturePhase::Update, Some(_)) if self.kind == GestureKind::Hold => {
                Err("a hold gesture has no update".to_owned())
            }
            _ => Ok(()),
        }
    }
}

impl Hazel {
    pub fn send_gesture(
        &mut self,
        pointer: &PointerHandle<Hazel>,
        mut gesture: Gesture,
        time: u32,
    ) -> Result<(), String> {
        let active = self.compositor.pointers.get(pointer).and_then(|state| state.gesture);
        gesture.check_order(active.as_ref())?;
        if let Some(active) = &active {
            gesture.fingers = active.fingers;
        }
        self.notify_idle_activity();
        let forwarded = match gesture.phase {
            GesturePhase::Begin => !self.emit_gesture_event(pointer, gesture),
            _ => {
                self.emit_gesture_event(pointer, gesture);
                active.is_some_and(|active| active.forwarded)
            }
        };
        self.track_gesture(pointer, gesture, forwarded);
        if forwarded {
            self.forward_gesture(pointer, gesture, time);
        }
        Ok(())
    }

    fn emit_gesture_event(&self, pointer: &PointerHandle<Hazel>, gesture: Gesture) -> bool {
        let event = PointerGestureEvent::new(gesture, pointer.clone(), self.compositor.modifiers_of_pointer(pointer));
        let prevented = event.default_prevented.clone();
        let _ = self.wm().input.events.emit(PointerGestureEvent::name(), event);
        prevented.get()
    }

    fn track_gesture(&mut self, pointer: &PointerHandle<Hazel>, gesture: Gesture, forwarded: bool) {
        let Some(state) = self.compositor.pointers.get_mut(pointer) else {
            return;
        };
        state.gesture = match gesture.phase {
            GesturePhase::Begin => Some(ActiveGesture { kind: gesture.kind, fingers: gesture.fingers, forwarded }),
            GesturePhase::Update => state.gesture,
            GesturePhase::End => None,
        };
    }

    fn forward_gesture(&mut self, pointer: &PointerHandle<Hazel>, gesture: Gesture, time: u32) {
        let serial = SERIAL_COUNTER.next_serial();
        let (fingers, delta, cancelled) = (gesture.fingers, gesture.delta, gesture.cancelled);
        match (gesture.kind, gesture.phase) {
            (GestureKind::Swipe, GesturePhase::Begin) => {
                pointer.gesture_swipe_begin(self, &GestureSwipeBeginEvent { serial, time, fingers })
            }
            (GestureKind::Swipe, GesturePhase::Update) => {
                pointer.gesture_swipe_update(self, &SwipeUpdate { time, delta })
            }
            (GestureKind::Swipe, GesturePhase::End) => {
                pointer.gesture_swipe_end(self, &GestureSwipeEndEvent { serial, time, cancelled })
            }
            (GestureKind::Pinch, GesturePhase::Begin) => {
                pointer.gesture_pinch_begin(self, &GesturePinchBeginEvent { serial, time, fingers })
            }
            (GestureKind::Pinch, GesturePhase::Update) => pointer.gesture_pinch_update(
                self,
                &PinchUpdate { time, delta, scale: gesture.scale, rotation: gesture.rotation },
            ),
            (GestureKind::Pinch, GesturePhase::End) => {
                pointer.gesture_pinch_end(self, &GesturePinchEndEvent { serial, time, cancelled })
            }
            (GestureKind::Hold, GesturePhase::Begin) => {
                pointer.gesture_hold_begin(self, &GestureHoldBeginEvent { serial, time, fingers })
            }
            (GestureKind::Hold, GesturePhase::End) => {
                pointer.gesture_hold_end(self, &GestureHoldEndEvent { serial, time, cancelled })
            }
            (GestureKind::Hold, GesturePhase::Update) => {}
        }
        pointer.frame(self);
    }

    pub fn on_gesture_begin<E: GestureBeginEvent<B>, B: smithay::backend::input::InputBackend>(
        &mut self,
        kind: GestureKind,
        event: &E,
    ) {
        let Some((_, pointer)) = self.compositor.get_pointer_handle(&event.device().id()) else {
            return;
        };
        let mut gesture = Gesture::new(kind, GesturePhase::Begin);
        gesture.fingers = event.fingers();
        let _ = self.send_gesture(&pointer, gesture, event.time_msec());
    }

    pub fn on_gesture_end<E: GestureEndEvent<B>, B: smithay::backend::input::InputBackend>(
        &mut self,
        kind: GestureKind,
        event: &E,
    ) {
        let Some((_, pointer)) = self.compositor.get_pointer_handle(&event.device().id()) else {
            return;
        };
        let mut gesture = Gesture::new(kind, GesturePhase::End);
        gesture.cancelled = event.cancelled();
        let _ = self.send_gesture(&pointer, gesture, event.time_msec());
    }

    pub fn on_swipe_update<B: smithay::backend::input::InputBackend>(&mut self, event: &B::GestureSwipeUpdateEvent) {
        let Some((_, pointer)) = self.compositor.get_pointer_handle(&event.device().id()) else {
            return;
        };
        let mut gesture = Gesture::new(GestureKind::Swipe, GesturePhase::Update);
        gesture.delta = (event.delta_x(), event.delta_y()).into();
        let _ = self.send_gesture(&pointer, gesture, event.time_msec());
    }

    pub fn on_pinch_update<B: smithay::backend::input::InputBackend>(&mut self, event: &B::GesturePinchUpdateEvent) {
        let Some((_, pointer)) = self.compositor.get_pointer_handle(&event.device().id()) else {
            return;
        };
        let mut gesture = Gesture::new(GestureKind::Pinch, GesturePhase::Update);
        gesture.delta = (event.delta_x(), event.delta_y()).into();
        gesture.scale = event.scale();
        gesture.rotation = event.rotation();
        let _ = self.send_gesture(&pointer, gesture, event.time_msec());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn active(kind: GestureKind) -> ActiveGesture {
        ActiveGesture { kind, fingers: 3, forwarded: true }
    }

    #[test]
    fn a_gesture_begins_only_when_none_is_in_progress() {
        let begin = Gesture::new(GestureKind::Swipe, GesturePhase::Begin);
        assert!(begin.check_order(None).is_ok());
        assert!(begin.check_order(Some(&active(GestureKind::Swipe))).is_err());
    }

    #[test]
    fn updates_and_ends_need_a_gesture_of_the_same_kind() {
        let update = Gesture::new(GestureKind::Swipe, GesturePhase::Update);
        assert!(update.check_order(None).is_err());
        assert!(update.check_order(Some(&active(GestureKind::Pinch))).is_err());
        assert!(update.check_order(Some(&active(GestureKind::Swipe))).is_ok());
        let end = Gesture::new(GestureKind::Pinch, GesturePhase::End);
        assert!(end.check_order(Some(&active(GestureKind::Pinch))).is_ok());
    }

    #[test]
    fn a_hold_has_no_update() {
        let update = Gesture::new(GestureKind::Hold, GesturePhase::Update);
        assert!(update.check_order(Some(&active(GestureKind::Hold))).is_err());
    }
}
