use smithay::{
    backend::input::{
        AbsolutePositionEvent, Axis, AxisRelativeDirection, ButtonState, Device, Event,
        InputBackend, PointerAxisEvent, PointerButtonEvent as SmithayPointerButtonEvent,
        PointerMotionEvent,
    },
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

        if let Some((window, surface, _)) =
            self.compositor.surface_under(pointer.current_location())
        {
            self.compositor.space.raise_element(&window, true);
            if let Some(keyboard) = seat.get_keyboard() {
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

        let previous_output_position = self
            .compositor
            .pointer_absolute_previous
            .get(&pointer_handle)
            .cloned()
            .unwrap_or_else(|| pointer_handle.current_location());

        let mapping = self
            .compositor
            .pointer_mapping
            .get(&pointer_handle)
            .unwrap_or(&PointerAbsoluteMapping::FirstContainingOutput);

        let coordinate_space = match mapping {
            PointerAbsoluteMapping::FirstContainingOutput => self
                .compositor
                .space
                .output_under(pointer_handle.current_location())
                .next()
                .or_else(|| self.compositor.space.outputs().next())
                .and_then(|o| self.compositor.space.output_geometry(o))
                .unwrap_or_default(),

            PointerAbsoluteMapping::Output(output) => self
                .compositor
                .space
                .output_geometry(output)
                .unwrap_or_default(),

            PointerAbsoluteMapping::Space(rect) => *rect,
        };

        let serial = SERIAL_COUNTER.next_serial();
        let location_output = event.position_transformed(coordinate_space.size);
        let location = location_output + coordinate_space.loc.to_f64();
        let utime = event.time();

        let under = self.compositor.surface_under(location);

        let delta = location_output - previous_output_position;

        self.compositor
            .pointer_absolute_previous
            .insert(pointer_handle.clone(), location_output);

        let event = PointerMoveEvent {
            default_prevented: Default::default(),
            pointer: pointer_handle.clone(),
            delta,
            delta_unaccel: delta,
            output_position: Some(location_output),
            position: location,
            serial,
            utime,
        };

        pointer_handle.motion(
            self,
            under.map(|(_, surface, point)| (surface, point)),
            &event.motion(),
        );

        pointer_handle.frame(self);

        self.wm()
            .input
            .events
            .emit(PointerMoveEvent::name(), event.clone())
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
