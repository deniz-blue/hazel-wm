use std::sync::Arc;

use mlua::UserData;

use crate::lua::api::wm_input::WmInput;

#[derive(Default)]
pub struct Wm {
	pub input: Arc<WmInput>,
}

impl UserData for Wm {
	fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
		fields.add_field_method_get("name", |_, _| Ok("Hazel"));
		fields.add_field_method_get("input", |_, this| Ok(this.input.clone()));
	}
}
