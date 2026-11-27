use std::{cell::Cell, error::Error as StdError, rc::Rc};

use smithay::{
    backend::input::{
        AbsolutePositionEvent, Axis, ButtonState, Device, Event,
        InputBackend, PointerAxisEvent, PointerButtonEvent as SmithayPointerButtonEvent,
        PointerMotionEvent,
    },
    input::pointer::PointerHandle,
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::{Logical, Point, SERIAL_COUNTER},
};

use crate::{
    core::{
        Hazel,
        input::{
            pointer::{PointerAbsoluteMapping, PointerState},
            pointer_constraints::{ConstraintEffect, clamp_to},
            scroll::Scroll,
        },
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
        let Some((_, pointer)) = self.compositor.get_pointer_handle(&device_id) else {
            return Ok(());
        };

        self.press_pointer_button(&pointer, event.button_code(), event.state(), event.time())
    }

    pub fn press_pointer_button(
        &mut self,
        pointer: &PointerHandle<Hazel>,
        button: u32,
        state: ButtonState,
        utime: u64,
    ) -> std::result::Result<(), Box<dyn StdError>> {
        self.notify_idle_activity();
        let serial = SERIAL_COUNTER.next_serial();

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
            modifiers: self.compositor.modifiers_of_pointer(pointer),
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
            self.wm()
                .input
                .events
                .emit(PointerButtonEvent::forwarded_name(), event)
                .into_box()?;
        }

        Ok(())
    }

    pub fn on_pointer_motion<B: InputBackend>(
        &mut self,
        event: B::PointerMotionEvent,
    ) -> std::result::Result<(), Box<dyn StdError>> {
        let device_id = event.device().id();
        let Some((_, pointer)) = self.compositor.get_pointer_handle(&device_id) else {
            return Ok(());
        };

        let event = PointerMoveEvent {
            default_prevented: Rc::new(Cell::new(false)),
            pointer: pointer.clone(),
            serial: SERIAL_COUNTER.next_serial(),
            modifiers: self.compositor.modifiers_of_pointer(&pointer),
            utime: event.time(),
            delta: event.delta(),
            delta_unaccel: event.delta_unaccel(),
            output_position: None,
            position: pointer.current_location() + event.delta(),
        };
        self.send_pointer_motion(event)
    }

    pub fn move_pointer(
        &mut self,
        pointer: &PointerHandle<Hazel>,
        position: Point<f64, Logical>,
        utime: u64,
    ) -> std::result::Result<(), Box<dyn StdError>> {
        let delta = position - pointer.current_location();
        self.compositor
            .pointers
            .entry(pointer.clone())
            .or_insert_with(PointerState::new)
            .absolute_previous = position;
        self.send_pointer_motion(PointerMoveEvent {
            default_prevented: Rc::new(Cell::new(false)),
            pointer: pointer.clone(),
            serial: SERIAL_COUNTER.next_serial(),
            modifiers: self.compositor.modifiers_of_pointer(pointer),
            utime,
            delta,
            delta_unaccel: delta,
            output_position: None,
            position,
        })
    }

    fn send_pointer_motion(
        &mut self,
        mut event: PointerMoveEvent,
    ) -> std::result::Result<(), Box<dyn StdError>> {
        self.notify_idle_activity();
        let pointer = event.pointer.clone();
        let held = self.compositor.input_under(pointer.current_location());
        let effect = self.constraint_effect(&pointer, held.as_ref());
        let locked = matches!(effect, ConstraintEffect::Locked);
        match &effect {
            ConstraintEffect::Locked => event.position = pointer.current_location(),
            ConstraintEffect::ConfinedTo(bounds) => event.position = clamp_to(*bounds, event.position),
            ConstraintEffect::Free => {}
        }
        // Lua first — prevent_default can skip the actual pointer op
        self.wm()
            .input
            .events
            .emit(PointerMoveEvent::name(), event.clone())
            .into_box()?;

        if !event.default_prevented.get() {
            self.forward_pointer_motion(&event, held, locked);
        }
        Ok(())
    }

    fn forward_pointer_motion(
        &mut self,
        event: &PointerMoveEvent,
        held: Option<(WlSurface, Point<f64, Logical>)>,
        locked: bool,
    ) {
        let pointer = &event.pointer;
        if locked {
            pointer.relative_motion(self, held, &event.relative_motion());
            pointer.frame(self);
            return;
        }
        let under = self.compositor.input_under(event.position);
        pointer.motion(self, under.clone(), &event.motion());
        pointer.relative_motion(self, under, &event.relative_motion());
        pointer.frame(self);
    }

    pub fn on_pointer_absolute<B: InputBackend>(
        &mut self,
        event: B::PointerMotionAbsoluteEvent,
    ) -> std::result::Result<(), Box<dyn StdError>> {
        let device_id = event.device().id();
        let Some((_, pointer_handle)) = self.compositor.get_pointer_handle(&device_id) else {
            return Ok(());
        };

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

        let delta = location_output - previous_output_position;

        self.compositor
            .pointers
            .entry(pointer_handle.clone())
            .or_insert_with(PointerState::new)
            .absolute_previous = location_output;

        self.send_pointer_motion(PointerMoveEvent {
            default_prevented: Rc::new(Cell::new(false)),
            pointer: pointer_handle.clone(),
            delta,
            delta_unaccel: delta,
            output_position: Some(location_output),
            position: location,
            serial,
            modifiers: self.compositor.modifiers_of_pointer(&pointer_handle),
            utime,
        })
    }

    pub fn on_pointer_axis<B: InputBackend>(
        &mut self,
        event: B::PointerAxisEvent,
    ) -> std::result::Result<(), Box<dyn StdError>> {
        let device_id = event.device().id();

        let Some((_, pointer)) = self.compositor.get_pointer_handle(&device_id) else {
            return Ok(());
        };

        let scroll = Scroll {
            source: event.source(),
            horizontal: event.amount(Axis::Horizontal),
            vertical: event.amount(Axis::Vertical),
            horizontal_v120: event.amount_v120(Axis::Horizontal),
            vertical_v120: event.amount_v120(Axis::Vertical),
        };
        self.send_scroll(&pointer, scroll, event.time_msec());
        Ok(())
    }

    pub fn send_scroll(&mut self, pointer: &PointerHandle<Hazel>, scroll: Scroll, time: u32) {
        self.notify_idle_activity();
        pointer.axis(self, scroll.axis_frame(time));
        pointer.frame(self);
    }
}
