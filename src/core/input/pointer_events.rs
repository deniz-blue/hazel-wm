use smithay::{
    backend::input::{
        AbsolutePositionEvent, Axis, AxisRelativeDirection, ButtonState, Device, Event,
        InputBackend, PointerAxisEvent, PointerButtonEvent as SmithayPointerButtonEvent,
        PointerMotionEvent,
    },
    desktop::WindowSurfaceType,
    input::pointer::AxisFrame,
    utils::SERIAL_COUNTER,
};
use std::error::Error as StdError;

use crate::{
    core::{Hazel, input::pointer::PointerAbsoluteMapping},
    lua::{
        api::wm_input_pointer::{PointerButtonEvent, PointerMoveEvent},
        ext::LuaErrorExt,
    },
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
        let utime = event.time();

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

        let event = PointerButtonEvent {
            button,
            state,
            serial,
            utime,
            pointer: pointer.clone(),
            default_prevented: Default::default(),
        };

        pointer.button(self, &event.button_event());

        pointer.frame(self);

        self.wm()
            .input
            .events
            .emit(PointerButtonEvent::name(), event)
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
        let utime = event.time();

        let (_, pointer) = self.compositor.get_pointer_handle(&device_id).unwrap();

        let event = PointerMoveEvent {
            default_prevented: Default::default(),
            pointer: pointer.clone(),
            serial: SERIAL_COUNTER.next_serial(),
            utime,
            delta,
            delta_unaccel,
            output_position: None,
            position: pointer.current_location() + delta,
        };

        pointer.relative_motion(self, None, &event.relative_motion());

        pointer.frame(self);

        self.wm()
            .input
            .events
            .emit(PointerMoveEvent::name(), event)
            .into_box()?;

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
        let utime = event.time();

        let pointer_over = self.compositor.surface_under(location);

        let delta = location - previous_position;

        let event = PointerMoveEvent {
            default_prevented: Default::default(),
            pointer: pointer_handle.clone(),
            delta,
            delta_unaccel: delta,
            output_position: Some(location),
            position: location,
            serial,
            utime,
        };

        pointer_handle.motion(self, pointer_over, &event.motion());

        pointer_handle.frame(self);

		self.wm()
			.input
			.events
			.emit(PointerMoveEvent::name(), event)
			.into_box()?;

        Ok(())
    }

    pub fn on_pointer_axis<B: InputBackend>(
        &mut self,
        event: B::PointerAxisEvent,
    ) -> std::result::Result<(), Box<dyn StdError>> {
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
