use std::collections::HashMap;

use mlua::{IntoLua, MetaMethod, UserData, Value};
use smithay::output::{Mode, WeakOutput};

use crate::{
    core::GlobalHazel,
    impl_lua_event_handler, impl_lua_event_source,
    lua::{
        api::utils::{LuaPoint, LuaSize},
        event_handler::LuaEventHandler,
    },
    lua_typedef,
};

#[derive(Default)]
pub struct WmOutputs {
    pub events: LuaEventHandler,
}

impl_lua_event_source!(WmOutputs);

impl UserData for WmOutputs {
    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        impl_lua_event_handler!(methods);

        methods.add_method("count", |_, _, _: ()| {
            GlobalHazel::with(|hazel| Ok(hazel.compositor.space.outputs().count()))
        });

        methods.add_method("name", |_, _, name: String| {
            GlobalHazel::with(|hazel| {
                let output = hazel
                    .compositor
                    .space
                    .outputs()
                    .find(|output| output.name() == name)
                    .map(|output| WmOutputHandle(output.downgrade()));

                Ok(output)
            })
        });

        methods.add_meta_method(MetaMethod::Pairs, |lua, _, (): ()| {
            let vec: Vec<_> = GlobalHazel::with(|hazel| {
                Ok(hazel
                    .compositor
                    .space
                    .outputs()
                    .map(|o| o.downgrade())
                    .collect())
            })?;
            let mut iter = vec.into_iter();
            let iterator_fn = lua.create_function_mut(move |lua, (): ()| match iter.next() {
                Some(weak_output) => {
                    let handle = WmOutputHandle(weak_output.clone());
                    Ok((
                        Some(
                            lua.create_string(
                                weak_output
                                    .upgrade()
                                    .map(|o| o.name())
                                    .unwrap_or_else(|| "Unknown".into()),
                            )?,
                        ),
                        Some(handle),
                    ))
                }
                None => Ok((None, None)),
            })?;

            Ok((iterator_fn, Value::Nil, Value::Nil))
        });
    }
}

lua_typedef!(WmOutputs => WmOutputs {
    fn count() -> number;
    fn name(name: string) -> WmOutput;
});

pub struct WmOutputHandle(WeakOutput);

impl WmOutputHandle {}

impl UserData for WmOutputHandle {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("name", |_, this| Ok(this.0.upgrade().map(|o| o.name())));
        fields.add_field_method_get("description", |_, this| {
            Ok(this.0.upgrade().map(|o| o.description()))
        });
        fields.add_field_method_get("mode", |_, this| {
            Ok(this
                .0
                .upgrade()
                .map(|o| o.current_mode().map(LuaOutputMode)))
        });
        fields.add_field_method_get("properties", |lua, this| {
            this.0
                .upgrade()
                .map(|o| -> Result<_, mlua::Error> {
                    let phys = o.physical_properties();
                    Ok(HashMap::from([
                        ("make".to_string(), phys.make.into_lua(lua)?),
                        ("model".to_string(), phys.model.into_lua(lua)?),
                        ("size".to_string(), LuaSize(phys.size).into_lua(lua)?),
                        (
                            "subpixel".to_string(),
                            format!("{:?}", phys.subpixel).into_lua(lua)?,
                        ),
                    ])
                    .into_lua(lua)?)
                })
                .transpose()
        });
    }

    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("position", |_, this, _: ()| {
            Ok(this.0.upgrade().map(|o| LuaPoint(o.current_location())))
        });
        methods.add_method("set_position", |_, this, point: LuaPoint<i32, _>| {
            if let Some(output) = this.0.upgrade() {
                output.change_current_state(None, None, None, Some(point.0));
                GlobalHazel::with(|hazel| {
                    hazel.compositor.space.map_output(&output, point.0);
                    Ok(())
                })?;
            }
            Ok(())
        });
    }
}

lua_typedef!(WmOutput => WmOutputHandle {
    let name: string;
    let description: string;
    let mode: OutputMode;
    let properties: table;
    fn position() -> Point;
    fn set_position(point: Point) -> nil;
});

pub struct LuaOutputMode(Mode);

impl IntoLua for LuaOutputMode {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<Value> {
        let table = lua.create_table()?;
        table.set("size", LuaSize(self.0.size).into_lua(lua)?)?;
        table.set("refresh", self.0.refresh.into_lua(lua)?)?;
        Ok(Value::Table(table))
    }
}

lua_typedef!(OutputMode => LuaOutputMode {
    let size: Size;
    let refresh: number;
});
