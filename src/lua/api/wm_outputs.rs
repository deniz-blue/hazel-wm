use std::collections::HashMap;

use mlua::{IntoLua, MetaMethod, UserData, Value};
use smithay::{output::WeakOutput, utils::Logical};

use crate::{
    core::GlobalHazel, impl_lua_event_handler, impl_lua_event_source, lua::{
        api::{
            shape_spec::ShapeSpec,
            utils::{LuaPoint, LuaRect, LuaSize},
            wm_shapes::{WmShape, add_output_shape},
            wm_output_modes::{LuaOutputMode, mode_list, parse_transform, transform_name},
        },
        event_handler::LuaEventHandler,
    }, lua_eq, lua_fromlua_userdataonly, lua_typedef
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
            GlobalHazel::try_with(|hazel| Ok(hazel.compositor.space.outputs().count()))
        });

        methods.add_method("under", |_, _, point: LuaPoint<f64, _>| {
            GlobalHazel::try_with(|hazel| {
                let outputs: Vec<_> = hazel
                    .compositor
                    .space
                    .output_under(point.0)
                    .map(|o| WmOutputHandle(o.downgrade()))
                    .collect();
                Ok(outputs)
            })
        });

        methods.add_method("add_virtual", |_, _, (name, size): (String, LuaSize<i32, Logical>)| {
            GlobalHazel::try_with(|hazel| hazel.add_virtual_output(&name, size.0).map(|output| WmOutputHandle::new(&output)))
        });

        methods.add_method("name", |_, _, name: String| {
            GlobalHazel::try_with(|hazel| {
                let output = hazel
                    .compositor
                    .space
                    .outputs()
                    .find(|output| output.name() == name)
                    .map(|output| WmOutputHandle(output.downgrade()));

                Ok(output)
            })
        });

        methods.add_method("get", |_, _, index: usize| {
            GlobalHazel::try_with(|hazel| {
                let idx = index.checked_sub(1);
                Ok(idx.and_then(|i| {
                    hazel
                        .compositor
                        .space
                        .outputs()
                        .nth(i)
                        .map(|o| WmOutputHandle(o.downgrade()))
                }))
            })
        });

        methods.add_method("all", |_, _, _: ()| {
            GlobalHazel::try_with(|hazel| {
                Ok(hazel
                    .compositor
                    .space
                    .outputs()
                    .map(|o| WmOutputHandle(o.downgrade()))
                    .collect::<Vec<_>>())
            })
        });

        methods.add_meta_method(MetaMethod::Pairs, |lua, _, (): ()| {
            let vec: Vec<_> = GlobalHazel::try_with(|hazel| {
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
    use added => WmOutput;
    use changed => WmOutput;
    use removed => WmOutput;
    fn add_virtual(name: string, size: Size) -> WmOutput;
    fn count() -> number;
    fn get(index: number) -> Option<WmOutput>;
    fn all() -> Array<WmOutput>;
    fn name(name: string) -> Option<WmOutput>;
    fn under(point: Point) -> Array<WmOutput>;
});

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WmOutputHandle(pub WeakOutput);

impl WmOutputHandle {
    pub fn new(output: &smithay::output::Output) -> Self {
        Self(output.downgrade())
    }
    pub fn name(&self) -> Option<String> {
        self.0.upgrade().map(|o| o.name())
    }

    pub fn description(&self) -> Option<String> {
        self.0.upgrade().map(|o| o.description())
    }

    fn describe(output: &smithay::output::Output, mode: smithay::output::Mode) -> LuaOutputMode {
        LuaOutputMode {
            mode,
            is_current: output.current_mode() == Some(mode),
            is_preferred: output.preferred_mode() == Some(mode),
        }
    }

    pub fn current_mode(&self) -> Option<LuaOutputMode> {
        let output = self.0.upgrade()?;
        output.current_mode().map(|mode| Self::describe(&output, mode))
    }

    pub fn modes(&self) -> Vec<LuaOutputMode> {
        let Some(output) = self.0.upgrade() else {
            return Vec::new();
        };
        output.modes().into_iter().map(|mode| Self::describe(&output, mode)).collect()
    }

    pub fn set_mode(&self, wanted: LuaOutputMode) -> mlua::Result<()> {
        let Some(output) = self.0.upgrade() else {
            return Ok(());
        };
        GlobalHazel::try_with(|hazel| hazel.set_output_mode(&output, wanted.mode).map_err(mlua::Error::external))
    }

    pub fn set_transform(&self, name: &str) -> mlua::Result<()> {
        let transform = parse_transform(name)?;
        if let Some(output) = self.0.upgrade() {
            GlobalHazel::try_with(|hazel| {
                hazel.set_output_transform(&output, transform);
                Ok(())
            })?;
        }
        Ok(())
    }

    pub fn position(&self) -> Option<LuaPoint<i32, Logical>> {
        self.0.upgrade().map(|o| LuaPoint(o.current_location()))
    }

    pub fn usable_area(&self) -> mlua::Result<Option<LuaRect>> {
        let Some(output) = self.0.upgrade() else {
            return Ok(None);
        };
        GlobalHazel::with(|hazel| hazel.compositor.usable_area(&output).map(LuaRect))
    }

    pub fn area(&self) -> mlua::Result<Option<LuaRect>> {
        let Some(output) = self.0.upgrade() else {
            return Ok(None);
        };
        GlobalHazel::with(|hazel| hazel.compositor.space.output_geometry(&output).map(LuaRect))
    }

    pub fn scale(&self) -> Option<f64> {
        self.0.upgrade().map(|o| o.current_scale().fractional_scale())
    }

    pub fn set_scale(&self, scale: f64) -> Result<(), mlua::Error> {
        if !(scale.is_finite() && scale >= 0.25 && scale <= 8.0) {
            return Err(mlua::Error::external(format!(
                "scale must be between 0.25 and 8, got {scale}"
            )));
        }
        if let Some(output) = self.0.upgrade() {
            GlobalHazel::try_with(|hazel| {
                hazel.set_output_scale(&output, scale);
                Ok(())
            })?;
        }
        Ok(())
    }

    pub fn set_position(&self, point: LuaPoint<f64, Logical>) -> mlua::Result<()> {
        let Some(output) = self.0.upgrade() else {
            return Ok(());
        };
        output.change_current_state(None, None, None, Some(point.0.to_i32_round()));
        GlobalHazel::with(|hazel| {
            hazel
                .compositor
                .space
                .map_output(&output, point.0.to_i32_round());
        })
    }
}

lua_fromlua_userdataonly!(WmOutputHandle);

impl UserData for WmOutputHandle {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("name", |_, this| Ok(this.0.upgrade().map(|o| o.name())));
        fields.add_field_method_get("description", |_, this| {
            Ok(this.0.upgrade().map(|o| o.description()))
        });
        fields.add_field_method_get("properties", |lua, this| {
            this.0
                .upgrade()
                .map(|o| -> Result<_, mlua::Error> {
                    let phys = o.physical_properties();
                    HashMap::from([
                        ("make".to_string(), phys.make.into_lua(lua)?),
                        ("model".to_string(), phys.model.into_lua(lua)?),
                        ("size".to_string(), LuaSize(phys.size).into_lua(lua)?),
                        (
                            "subpixel".to_string(),
                            format!("{:?}", phys.subpixel).into_lua(lua)?,
                        ),
                    ])
                    .into_lua(lua)
                })
                .transpose()
        });
    }

    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("position", |_, this, _: ()| Ok(this.position()));
        methods.add_method("set_position", |_, this, point: LuaPoint<_, _>| this.set_position(point));
        methods.add_method("scale", |_, this, _: ()| Ok(this.scale()));
        methods.add_method("set_scale", |_, this, scale: f64| this.set_scale(scale));
        methods.add_method("usable_area", |_, this, _: ()| this.usable_area());
        methods.add_method("area", |_, this, _: ()| this.area());
        methods.add_method("mode", |_, this, _: ()| Ok(this.current_mode()));
        methods.add_method("modes", |lua, this, _: ()| mode_list(lua, this.modes()));
        methods.add_method("set_mode", |_, this, mode: LuaOutputMode| this.set_mode(mode));
        methods.add_method("transform", |_, this, _: ()| {
            Ok(this.0.upgrade().map(|output| transform_name(output.current_transform())))
        });
        methods.add_method("set_transform", |_, this, name: String| this.set_transform(&name));
        methods.add_method("size", |_, this, _: ()| {
            Ok(this.current_mode().map(|mode| LuaSize(mode.mode.size)))
        });
        methods.add_method("remove", |_, this, _: ()| {
            let Some(output) = this.0.upgrade() else {
                return Ok(());
            };
            GlobalHazel::try_with(|hazel| hazel.remove_virtual_output(&output).map_err(mlua::Error::external))
        });
        methods.add_method("add_shape", |_, this, spec: ShapeSpec| -> mlua::Result<WmShape> {
            let output = this.0.upgrade().ok_or_else(|| mlua::Error::external("the output is gone"))?;
            add_output_shape(&output, spec)
        });

		lua_eq!(methods);
    }
}

lua_typedef!(WmOutput => WmOutputHandle {
    let name: string;
    let description: string;
    let properties: table;
    fn mode() -> OutputMode;
    fn modes() -> Array<OutputMode>;
    fn set_mode(mode: OutputMode) -> nil;
    fn transform() -> string;
    fn set_transform(name: string) -> nil;
    fn position() -> Point;
    fn set_position(point: Point) -> nil;
    fn scale() -> number;
    fn set_scale(scale: number) -> nil;
    fn size() -> Size;
    fn usable_area() -> Rect;
    fn area() -> Rect;
    fn add_shape(spec: ShapeSpec) -> Shape;
    fn remove() -> nil;
});

#[cfg(test)]
mod tests {
    use smithay::{
        output::{Mode, Output, PhysicalProperties, Subpixel},
        utils::Point,
    };

    use super::*;

    fn removed_output() -> WmOutputHandle {
        let output = Output::new(
            "gone".into(),
            PhysicalProperties { size: (0, 0).into(), subpixel: Subpixel::Unknown, make: String::new(), model: String::new(), serial_number: String::new() },
        );
        WmOutputHandle::new(&output)
    }

    #[test]
    fn a_removed_output_answers_nothing() {
        let handle = removed_output();
        assert_eq!(handle.name(), None);
        assert_eq!(handle.description(), None);
        assert!(handle.current_mode().is_none());
        assert!(handle.modes().is_empty());
        assert!(handle.position().is_none());
        assert_eq!(handle.scale(), None);
        assert!(matches!(handle.usable_area(), Ok(None)));
        assert!(matches!(handle.area(), Ok(None)));
    }

    #[test]
    fn changing_a_removed_output_does_nothing_and_fails_nothing() {
        let handle = removed_output();
        let mode = LuaOutputMode { mode: Mode { size: (800, 600).into(), refresh: 60000 }, is_current: false, is_preferred: false };
        assert!(handle.set_mode(mode).is_ok());
        assert!(handle.set_transform("90").is_ok());
        assert!(handle.set_scale(2.0).is_ok());
        assert!(handle.set_position(LuaPoint(Point::from((1.0, 2.0)))).is_ok());
    }
}
