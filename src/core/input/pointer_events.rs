use std::error::Error as StdError;
use smithay::{
    backend::input::{
        AbsolutePositionEvent, Axis, AxisRelativeDirection, ButtonState, Device, Event,
        InputBackend, PointerAxisEvent, PointerButtonEvent, PointerMotionEvent,
    },
    desktop::WindowSurfaceType,
    input::pointer::{AxisFrame, ButtonEvent, MotionEvent, RelativeMotionEvent},
    utils::SERIAL_COUNTER,
};

use crate::{
    core::{Hazel, input::pointer::PointerAbsoluteMapping},
    lua::{api::wm_input_pointer::LuaPointerButtonEvent, ext::LuaErrorExt},
};

impl Hazel {
    pub fn on_pointer_button<B: InputBackend>(
        &mut self,
        event: B::PointerButtonEvent,
    ) -> std::result::Result<(), Box<dyn StdError>> {
        let device_id = event.device().id();
        let button = event.button_code();
        let state = event.state();
        let serial = SERIAL_COUNTER.next_serial();
        let time = event.time_msec();

        let (seat, pointer) = self.compositor.get_pointer_handle(&device_id).unwrap();

        if let Some((window, p)) = self.compositor.window_under(pointer.current_location()) {
            let surface = window.surface_under(p.to_f64(), WindowSurfaceType::all());
            if let (Some((surface, _)), Some(keyboard)) = (surface, seat.get_keyboard()) {
                keyboard.set_focus(self, Some(surface), serial);
            }
        } else if let Some(keyboard) = seat.get_keyboard() {
            keyboard.set_focus(self, None, serial);
        }

        let pressed = self
            .compositor
            .pointer_pressed
            .entry(pointer.clone())
            .or_default();

        if state == ButtonState::Pressed {
            pressed.push(button);
        } else {
            pressed.retain(|&b| b != button);
        }

        let button_event = ButtonEvent {
            button,
            state,
            serial,
            time,
        };

        pointer.button(self, &button_event);

        pointer.frame(self);

        let event = LuaPointerButtonEvent {
            event: button_event,
            pointer: pointer.clone(),
            default_prevented: Default::default(),
        };

        self.wm()
            .input
            .events
            .emit(LuaPointerButtonEvent::name(), event)
            .into_box()?;

        Ok(())
    }

    pub fn on_pointer_motion<B: InputBackend>(
        &mut self,
        event: B::PointerMotionEvent,
    ) -> std::result::Result<(), Box<dyn StdError>> {
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

        Ok(())
    }

    pub fn on_pointer_absolute<B: InputBackend>(
        &mut self,
        event: B::PointerMotionAbsoluteEvent,
    ) -> std::result::Result<(), Box<dyn StdError>> {
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

        pointer_handle.frame(self);

        Ok(())
    }

    pub fn on_pointer_axis<B: InputBackend>(&mut self, event: B::PointerAxisEvent) -> std::result::Result<(), Box<dyn StdError>> {
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

        Ok(())
    }
}
