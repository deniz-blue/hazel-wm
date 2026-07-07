#[macro_export]
macro_rules! lua_concat {
    ($methods:ident) => {
        $methods.add_meta_function(
            mlua::MetaMethod::Concat,
            |lua, (left, right): (mlua::Value, mlua::Value)| {
                let tostring: mlua::Function = lua.globals().get("tostring")?;
                let left_str: String = tostring.call(left)?;
                let right_str: String = tostring.call(right)?;

                Ok(format!("{}{}", left_str, right_str))
            },
        )
    };
}

#[macro_export]
macro_rules! lua_eq {
    ($methods:ident) => {
        $methods.add_meta_function(mlua::MetaMethod::Eq, |_, (left, right): (Self, Self)| {
            Ok(left == right)
        })
    };
}

#[macro_export]
macro_rules! lua_fromlua_userdataonly {
    ($type:ident) => {
        impl mlua::FromLua for $type {
            fn from_lua(value: mlua::Value, _: &mlua::Lua) -> mlua::Result<Self> {
                match value {
                    mlua::Value::UserData(ud) => {
                        let window = ud.borrow::<$type>()?;
                        Ok(window.clone())
                    }
                    _ => Err(mlua::Error::FromLuaConversionError {
                        from: "Value",
                        to: String::from(stringify!($type)),
                        message: Some(format!("Expected a {} userdata", stringify!($type))),
                    }),
                }
            }
        }
    };
}

#[macro_export]
macro_rules! lua_debug_metamethods {
    ($methods:ident) => {
        $methods.add_meta_method(mlua::MetaMethod::ToString, |_, this, _: ()| {
            Ok(format!("{this:?}"))
        });

        $crate::lua_concat!($methods);
    };
}

#[macro_export]
macro_rules! lua_display_metamethods {
    ($methods:ident) => {
        $methods.add_meta_method(mlua::MetaMethod::ToString, |_, this, _: ()| {
            Ok(format!("{this}"))
        });

        $crate::lua_concat!($methods);
    };
}
