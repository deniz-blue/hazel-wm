use std::rc::Rc;

use mlua::{Error, UserData};

use crate::{core::GlobalHazel, lua::api::wm_input::WmInput};

pub struct Wm {
    pub input: Rc<WmInput>,
}

impl Wm {
    pub fn new() -> Self {
        Self {
            input: Rc::new(WmInput::new()),
        }
    }
}

impl UserData for Wm {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("name", |_, _| Ok("Hazel"));
        fields.add_field_method_get("input", |_, this| Ok(this.input.clone()));
    }

    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("doohickey", |_, _this, ()| {
            println!("Doohickey():");
            GlobalHazel::with(|hazel| Ok(hazel.doohickey()))
        });
    }
}
