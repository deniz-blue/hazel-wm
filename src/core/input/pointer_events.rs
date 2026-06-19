use std::{cell::Cell, error::Error as StdError, rc::Rc};

use smithay::{
    backend::input::{
        AbsolutePositionEvent, Axis, AxisRelativeDirection, ButtonState, Device, Event,
        InputBackend, PointerAxisEvent, PointerButtonEvent as SmithayPointerButtonEvent,
        PointerMotionEvent,
    },
    input::pointer::AxisFrame,
    utils::SERIAL_COUNTER,
    wayland::shell::wlr_layer::Layer,
};

use crate::{
    core::{
        Hazel,
        input::pointer::{PointerAbsoluteMapping, PointerState},
    },
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
        let pointer_pos = pointer.current_location();

        // Raise for windows (only if no top layer consumed the position)
        if self
            .compositor
            .layer_under(pointer_pos, &[Layer::Overlay, Layer::Top])
            .is_none()
        {
            if let Some((window, _, _)) = self.compositor.surface_under(pointer_pos) {
                self.compositor.space.raise_element(&window, true);
            }
        }

        if let Some((surface, _)) = self.compositor.input_under(pointer_pos) {
            if let Some(keyboard) = seat.get_keyboard() {
                keyboard.set_focus(self, Some(surface), serial);
            }
        } else if let Some(keyboard) = seat.get_keyboard() {
            keyboard.set_focus(self, None, serial);
        }

        let pressed = &mut self
            .compositor
            .pointers
            .entry(pointer.clone())
            .or_insert_with(PointerState::new)
            .pressed;

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
            default_prevented: Rc::new(Cell::new(false)),
        };

        // Lua first — prevent_default can skip the actual pointer op
        self.wm()
            .input
            .events
            .emit(PointerButtonEvent::name(), event.clone())
            .into_box()?;

        if !event.default_prevented.get() {
            pointer.button(self, &event.button_event());
            pointer.frame(self);
        }

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

        let new_position = pointer.current_location() + delta;
        let under = self.compositor.input_under(new_position);

        let event = PointerMoveEvent {
            default_prevented: Rc::new(Cell::new(false)),
            pointer: pointer.clone(),
            serial: SERIAL_COUNTER.next_serial(),
            utime,
            delta,
            delta_unaccel,
            output_position: None,
            position: new_position,
        };

        // Lua first — prevent_default can skip the actual pointer op
        self.wm()
            .input
            .events
            .emit(PointerMoveEvent::name(), event.clone())
            .into_box()?;

        if !event.default_prevented.get() {
            pointer.motion(self, under, &event.motion());
            pointer.frame(self);
        }

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
            .pointers
            .get(&pointer_handle)
            .map(|s| s.absolute_previous)
            .unwrap_or_else(|| pointer_handle.current_location());

        let mapping = self
            .compositor
            .pointers
            .get(&pointer_handle)
            .map(|s| &s.mapping)
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

        let under = self.compositor.input_under(location);

        let delta = location_output - previous_output_position;

        self.compositor
            .pointers
            .entry(pointer_handle.clone())
            .or_insert_with(PointerState::new)
            .absolute_previous = location_output;

        let event = PointerMoveEvent {
            default_prevented: Rc::new(Cell::new(false)),
            pointer: pointer_handle.clone(),
            delta,
            delta_unaccel: delta,
            output_position: Some(location_output),
            position: location,
            serial,
            utime,
        };

        // Lua first — prevent_default can skip the actual pointer op
        self.wm()
            .input
            .events
            .emit(PointerMoveEvent::name(), event.clone())
            .into_box()?;

        if !event.default_prevented.get() {
            pointer_handle.motion(self, under, &event.motion());
            pointer_handle.frame(self);
        }

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
