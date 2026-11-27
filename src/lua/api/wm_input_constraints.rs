use mlua::UserData;
use smithay::{
    input::pointer::PointerHandle,
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    wayland::pointer_constraints::{PointerConstraint, with_pointer_constraint},
};

use crate::{
    core::{GlobalHazel, Hazel},
    lua::api::{wm_input_pointer::WmInputPointer, wm_windows::WmWindow},
    lua_typedef,
};

pub struct WmPointerConstraint {
    pub surface: WlSurface,
    pub pointer: PointerHandle<Hazel>,
}

impl WmPointerConstraint {
    pub fn name() -> String {
        String::from("pointer_constraint")
    }

    fn alive(&self) -> bool {
        with_pointer_constraint(&self.surface, &self.pointer, |constraint| constraint.is_some())
    }

    fn kind(&self) -> Option<&'static str> {
        with_pointer_constraint(&self.surface, &self.pointer, |constraint| {
            constraint.map(|constraint| match &*constraint {
                PointerConstraint::Locked(_) => "locked",
                PointerConstraint::Confined(_) => "confined",
            })
        })
    }

    fn active(&self) -> bool {
        with_pointer_constraint(&self.surface, &self.pointer, |constraint| {
            constraint.is_some_and(|constraint| constraint.is_active())
        })
    }

    fn activate(&self) {
        with_pointer_constraint(&self.surface, &self.pointer, |constraint| {
            if let Some(constraint) = constraint
                && !constraint.is_active()
            {
                constraint.activate();
            }
        });
    }

    fn deactivate(&self) -> mlua::Result<()> {
        GlobalHazel::try_with(|hazel| {
            with_pointer_constraint(&self.surface, &self.pointer, |constraint| {
                if let Some(constraint) = constraint
                    && constraint.is_active()
                {
                    constraint.deactivate(hazel, &self.surface, &self.pointer);
                }
            });
            Ok(())
        })
    }
}

impl UserData for WmPointerConstraint {
    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("alive", |_, this, _: ()| Ok(this.alive()));
        methods.add_method("kind", |_, this, _: ()| Ok(this.kind()));
        methods.add_method("active", |_, this, _: ()| Ok(this.active()));
        methods.add_method("activate", |_, this, _: ()| {
            this.activate();
            Ok(())
        });
        methods.add_method("deactivate", |_, this, _: ()| this.deactivate());
        methods.add_method("pointer", |_, this, _: ()| Ok(WmInputPointer(this.pointer.clone())));
        methods.add_method("window", |_, this, _: ()| {
            GlobalHazel::try_with(|hazel| {
                Ok(hazel
                    .compositor
                    .window_of_surface(&this.surface)
                    .map(WmWindow))
            })
        });
    }
}

lua_typedef!(PointerConstraint => WmPointerConstraint {
    fn alive() -> boolean;
    fn kind() -> Option<string>;
    fn active() -> boolean;
    fn activate() -> nil;
    fn deactivate() -> nil;
    fn pointer() -> Pointer;
    fn window() -> Option<Window>;
});
