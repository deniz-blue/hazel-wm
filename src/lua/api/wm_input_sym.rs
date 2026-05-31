use mlua::{MetaMethod, UserData, Value};
use smithay::input::keyboard::{
    Keysym,
    xkb::{KEYSYM_CASE_INSENSITIVE, keysym_from_name, keysym_get_name},
};

use crate::lua_concat;

pub struct LuaKeysym(pub Keysym);

impl UserData for LuaKeysym {
    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        lua_concat!(methods);

        methods.add_meta_method(MetaMethod::ToString, |_, this, _: ()| {
            Ok(keysym_get_name(this.0))
        });

        methods.add_meta_method(MetaMethod::Eq, |_, this, other: Value| match other {
            Value::String(lua_str) => {
                let str_val = lua_str.to_str()?;
                let target_sym = keysym_from_name(&str_val, KEYSYM_CASE_INSENSITIVE);
                Ok(this.0 == target_sym)
            }
            Value::Integer(int) => {
                let target_sym = Keysym::new(int as u32);
                Ok(this.0 == target_sym)
            }
            Value::UserData(ud) => {
                if let Ok(other_sym) = ud.borrow::<Self>() {
                    Ok(this.0 == other_sym.0)
                } else {
                    Ok(false)
                }
            }
            _ => Ok(false),
        });
    }
}

pub struct LuaMouseButton(pub u32);

// <linux/input-event-codes.h>.
const BTN_LEFT: u32 = 0x110;
const BTN_RIGHT: u32 = 0x111;
const BTN_MIDDLE: u32 = 0x112;
const BTN_SIDE: u32 = 0x113;
const BTN_EXTRA: u32 = 0x114;
const BTN_FORWARD: u32 = 0x115;
const BTN_BACK: u32 = 0x116;

impl LuaMouseButton {
    pub fn name(&self) -> Option<String> {
        match self.0 {
            BTN_LEFT => Some(String::from("Left")),
            BTN_RIGHT => Some(String::from("Right")),
            BTN_MIDDLE => Some(String::from("Middle")),
            BTN_BACK | BTN_SIDE => Some(String::from("Back")),
            BTN_FORWARD | BTN_EXTRA => Some(String::from("Forward")),
            _ => None,
        }
    }
}

impl UserData for LuaMouseButton {
    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        lua_concat!(methods);

        methods.add_meta_method(MetaMethod::Eq, |_, this, other: Value| match other {
            Value::String(lua_str) => {
                let str_val = lua_str.to_str()?;
                Ok(str_val
                    == this
                        .name()
                        .unwrap_or_else(|| format!("Unknown({})", this.0)))
            }
            Value::Integer(int) => Ok(this.0 == int as u32),
            Value::UserData(ud) => {
                if let Ok(other_btn) = ud.borrow::<Self>() {
                    Ok(this.0 == other_btn.0)
                } else {
                    Ok(false)
                }
            }
            _ => Ok(false),
        });

        methods.add_meta_method(MetaMethod::ToString, |_, this, _: ()| {
            Ok(this
                .name()
                .unwrap_or_else(|| format!("Unknown({})", this.0)))
        });
    }
}

pub struct LuaKeys;

impl UserData for LuaKeys {
    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::Index, |_, _, key: String| {
            Ok(LuaKeysym(keysym_from_name(&key, KEYSYM_CASE_INSENSITIVE)))
        });
    }
}

pub struct LuaMouseButtons;

impl UserData for LuaMouseButtons {
    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::Index, |_, _, key: String| {
            Ok(LuaMouseButton(match key.as_str() {
                "Left" => BTN_LEFT,
                "Right" => BTN_RIGHT,
                "Middle" => BTN_MIDDLE,
                "Back" => BTN_BACK,
                "Forward" => BTN_FORWARD,
                _ => 0,
            }))
        });
    }
}
