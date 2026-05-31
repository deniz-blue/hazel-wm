use smithay::{
    backend::input::{
        AbsolutePositionEvent, Axis, AxisRelativeDirection, Device, Event, InputBackend,
        PointerAxisEvent, PointerButtonEvent, PointerMotionEvent,
    },
    desktop::WindowSurfaceType,
    input::pointer::{AxisFrame, ButtonEvent, MotionEvent, RelativeMotionEvent},
    utils::SERIAL_COUNTER,
};

use crate::core::{Hazel, input::pointer::PointerAbsoluteMapping};

impl Hazel {
    pub fn on_pointer_button<B: InputBackend>(&mut self, event: B::PointerButtonEvent) {
        let device_id = event.device().id();
        let button = event.button_code();
        let state = event.state();
        let serial = SERIAL_COUNTER.next_serial();
        let time = event.time_msec();

        let (seat, pointer) = self.compositor.get_pointer_handle(&device_id).unwrap();

        if let Some((window, p)) = self.compositor.window_under(pointer.current_location()) {
            // window.set_activated(true);
            let surface = window.surface_under(p.to_f64(), WindowSurfaceType::all());
            if let (Some((surface, _)), Some(keyboard)) = (surface, seat.get_keyboard()) {
                keyboard.set_focus(self, Some(surface), serial);
            }
        } else if let Some(keyboard) = seat.get_keyboard() {
            keyboard.set_focus(self, None, serial);
        }

        pointer.button(
            self,
            &ButtonEvent {
                button,
                state,
                serial,
                time,
            },
        );

        pointer.frame(self);
    }

    pub fn on_pointer_motion<B: InputBackend>(&mut self, event: B::PointerMotionEvent) {
        let device_id = event.device().id();
        let delta = event.delta();
        let delta_unaccel = event.delta_unaccel();
        let time = event.time();

        let (_, pointer) = self.compositor.get_pointer_handle(&device_id).unwrap();

        pointer.relative_motion(
            self,
            None,
            &RelativeMotionEvent {
                delta,
                delta_unaccel,
                utime: time,
            },
        );

        pointer.frame(self);
    }

    pub fn on_pointer_absolute<B: InputBackend>(
        &mut self,
        event: B::PointerMotionAbsoluteEvent,
    ) {
        let device_id = event.device().id();
        let (_, pointer_handle) = self.compositor.get_pointer_handle(&device_id).unwrap();
        let previous_position = pointer_handle.current_location();

        let mapping = self
            .compositor
            .pointer_mapping
            .get(&device_id)
            .unwrap_or(&PointerAbsoluteMapping::FirstContainingOutput);

        let coordinate_space = match mapping {
            PointerAbsoluteMapping::FirstContainingOutput => self
                .compositor
                .space
                .output_under(previous_position)
                .next()
                .or_else(|| self.compositor.space.outputs().next())
                .and_then(|o| self.compositor.space.output_geometry(o))
                .map(|geo| geo.size)
                .unwrap_or((0, 0).into()),

            PointerAbsoluteMapping::Output(output) => self
                .compositor
                .space
                .output_geometry(output)
                .map(|geo| geo.size)
                .unwrap_or((0, 0).into()),

            PointerAbsoluteMapping::Space(size) => *size,
        };

        let serial = SERIAL_COUNTER.next_serial();
        let location = event.position_transformed(coordinate_space);
        let time = event.time_msec();

        let pointer_over = self.compositor.surface_under(location);

        pointer_handle.motion(
            self,
            pointer_over,
            &MotionEvent {
                location,
                serial,
                time,
            },
        );
    }

    pub fn on_pointer_axis<B: InputBackend>(&mut self, event: B::PointerAxisEvent) {
        let device_id = event.device().id();

        let (_, pointer) = self.compositor.get_pointer_handle(&device_id).unwrap();

        pointer.axis(
            self,
            AxisFrame {
                axis: (
                    event.amount(Axis::Horizontal).unwrap_or_default(),
                    event.amount(Axis::Vertical).unwrap_or_default(),
                ),
                source: Some(event.source()),
                time: event.time_msec(),
                stop: (false, false),
                relative_direction: (
                    AxisRelativeDirection::Identical,
                    AxisRelativeDirection::Identical,
                ),
                v120: None, // eeh later
            },
        );

        pointer.frame(self);
    }
}
