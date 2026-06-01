pub trait LuaTypeDef {
    fn name() -> &'static str;
    fn dump();
}

#[macro_export]
macro_rules! lua_typedef {
    (
        $class:ident => $impl_into:ty {
			$( extern $global:ident ; )*
            $( let $field:ident : $field_ty:ty ; )*
            $( use $name:ident => $type:ty ; )*
            $( fn $method:ident ( $($param:ident : $param_tt:tt),* $(,)? ) -> $ret:ty ; )*
        }
    ) => {
		impl $crate::lua::typedefs::LuaTypeDef for $impl_into {
			fn name() -> &'static str {
				stringify!($class)
			}

			fn dump() {
				println!("--- @class {}", stringify!($class));
				$(
					println!("--- @field {} {}", stringify!($field), stringify!($field_ty));
				)*
				println!("local {} = {{}}\n", stringify!($class));

				$(
					println!("{} = {}", stringify!($global), stringify!($class));
				)*

				$(
					$(
						println!("--- @param {} {}", stringify!($param), stringify!($param_tt));
					)*
					println!("--- @return {}", stringify!($ret));
					print!("function {}:{}(", stringify!($class), stringify!($method));

					let mut _first = true;
					$(
						if !_first { print!(", "); }
						print!("{}", stringify!($param));
						_first = false;
					)*
					println!(") end\n");
				)*

				$(
					println!("--- @param event \"{}\"", stringify!($name));
					println!("--- @param callback fun(e: {})", stringify!($type));
					println!("function {}:on(event, callback) end\n", stringify!($class));
				)*
			}
		}
    };
}
