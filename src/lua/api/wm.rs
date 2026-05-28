use std::rc::Rc;

use mlua::UserData;

use crate::lua::{HazelHandle, api::wm_input::WmInput};

pub struct Wm {
	pub hazel: HazelHandle,
    pub input: Rc<WmInput>,
}

impl Wm {
    pub fn new(hazel: HazelHandle) -> Self {
        Self {
			hazel: hazel.clone(),
            input: Rc::new(WmInput::new(hazel)),
        }
    }
}

impl UserData for Wm {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("name", |_, _| Ok("Hazel"));
        fields.add_field_method_get("input", |_, this| Ok(this.input.clone()));
    }

	fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
		methods.add_method("doohickey", |_, this, ()| {
			println!("Doohickey():");
			this.hazel.borrow_mut().doohickey();
			Ok(())
		});
	}
}
