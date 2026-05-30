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
macro_rules! lua_debug_metamethods {
    ($methods:ident) => {
        $methods.add_meta_method(mlua::MetaMethod::ToString, |_, this, _: ()| {
            Ok(format!("{this:?}"))
        });

        crate::lua_concat!($methods);
    };
}

#[macro_export]
macro_rules! lua_display_metamethods {
    ($methods:ident) => {
        $methods.add_meta_method(mlua::MetaMethod::ToString, |_, this, _: ()| {
            Ok(format!("{this}"))
        });

        crate::lua_concat!($methods);
    };
}
