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

pub struct LuaKeys;

impl UserData for LuaKeys {
    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::Index, |_, _, key: String| {
            Ok(LuaKeysym(keysym_from_name(&key, KEYSYM_CASE_INSENSITIVE)))
        });
    }
}
