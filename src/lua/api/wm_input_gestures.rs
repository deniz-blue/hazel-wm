use std::{cell::Cell, rc::Rc};

use mlua::{Table, UserData};
use smithay::{
    input::{keyboard::ModifiersState, pointer::PointerHandle},
    utils::Logical,
};

use crate::{
    core::{
        Hazel,
        input::gestures::{Gesture, GestureKind, GesturePhase},
    },
    lua::api::{
        utils::LuaPoint, wm_input_keyboard::ModifiersStateUserData, wm_input_pointer::WmInputPointer,
    },
    lua_typedef,
};

const KINDS: [(&str, GestureKind); 3] =
    [("swipe", GestureKind::Swipe), ("pinch", GestureKind::Pinch), ("hold", GestureKind::Hold)];
const PHASES: [(&str, GesturePhase); 3] =
    [("begin", GesturePhase::Begin), ("update", GesturePhase::Update), ("end", GesturePhase::End)];

fn name_of<T: PartialEq + Copy>(table: &[(&'static str, T)], value: T) -> &'static str {
    table.iter().find(|(_, candidate)| *candidate == value).map_or("", |(name, _)| name)
}

fn parse<T: Copy>(table: &[(&str, T)], what: &str, name: &str) -> mlua::Result<T> {
    table.iter().find(|(candidate, _)| *candidate == name).map(|(_, value)| *value).ok_or_else(|| {
        let names: Vec<_> = table.iter().map(|(name, _)| *name).collect();
        mlua::Error::external(format!("unknown gesture {what} {name:?}, expected one of {}", names.join(", ")))
    })
}

pub fn parse_gesture(kind: &str, phase: &str, options: Option<Table>) -> mlua::Result<Gesture> {
    let mut gesture = Gesture::new(parse(&KINDS, "kind", kind)?, parse(&PHASES, "phase", phase)?);
    if let Some(options) = options {
        gesture.fingers = options.get::<Option<u32>>("fingers")?.unwrap_or(gesture.fingers);
        if let Some(delta) = options.get::<Option<LuaPoint<f64, Logical>>>("delta")? {
            gesture.delta = delta.0;
        }
        gesture.scale = options.get::<Option<f64>>("scale")?.unwrap_or(gesture.scale);
        gesture.rotation = options.get::<Option<f64>>("rotation")?.unwrap_or(gesture.rotation);
        gesture.cancelled = options.get::<Option<bool>>("cancelled")?.unwrap_or(false);
    }
    Ok(gesture)
}

#[derive(Clone)]
pub struct PointerGestureEvent {
    pub gesture: Gesture,
    pub pointer: PointerHandle<Hazel>,
    pub modifiers: ModifiersState,
    pub default_prevented: Rc<Cell<bool>>,
}

impl PointerGestureEvent {
    pub fn new(gesture: Gesture, pointer: PointerHandle<Hazel>, modifiers: ModifiersState) -> Self {
        Self { gesture, pointer, modifiers, default_prevented: Rc::new(Cell::new(false)) }
    }

    pub fn name() -> String {
        String::from("pointer_gesture")
    }
}

impl UserData for PointerGestureEvent {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("kind", |_, this| Ok(name_of(&KINDS, this.gesture.kind)));
        fields.add_field_method_get("phase", |_, this| Ok(name_of(&PHASES, this.gesture.phase)));
        fields.add_field_method_get("fingers", |_, this| Ok(this.gesture.fingers));
        fields.add_field_method_get("delta", |_, this| Ok(LuaPoint(this.gesture.delta)));
        fields.add_field_method_get("scale", |_, this| Ok(this.gesture.scale));
        fields.add_field_method_get("rotation", |_, this| Ok(this.gesture.rotation));
        fields.add_field_method_get("cancelled", |_, this| Ok(this.gesture.cancelled));
        fields.add_field_method_get("pointer", |_, this| Ok(WmInputPointer(this.pointer.clone())));
        fields.add_field_method_get("modifiers", |_, this| Ok(ModifiersStateUserData(this.modifiers)));
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("prevent_default", |_, this, _: ()| {
            this.default_prevented.set(true);
            Ok(())
        });
    }
}

lua_typedef!(PointerGestureEvent => PointerGestureEvent {
    let kind: string;
    let phase: string;
    let fingers: number;
    let delta: Point;
    let scale: number;
    let rotation: number;
    let cancelled: boolean;
    let pointer: Pointer;
    let modifiers: Modifiers;
    fn prevent_default() -> nil;
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_and_phases_are_matched_by_name() {
        let gesture = parse_gesture("pinch", "update", None).unwrap();
        assert_eq!((gesture.kind, gesture.phase), (GestureKind::Pinch, GesturePhase::Update));
        let message = parse_gesture("tap", "begin", None).err().unwrap().to_string();
        assert!(message.contains("swipe, pinch, hold"), "{message}");
    }
}
