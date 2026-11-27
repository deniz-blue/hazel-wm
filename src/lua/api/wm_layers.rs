use mlua::{Lua, Table, UserData};
use smithay::{
    desktop::{LayerSurface as DesktopLayerSurface, layer_map_for_output},
    utils::Logical,
    wayland::shell::wlr_layer::{Anchor, KeyboardInteractivity, Layer as LayerDepth},
};

use crate::{
    core::{
        GlobalHazel,
        handlers::layer_shell::LayerOutput,
        layer_properties::{LayerProperties, edge_name},
    },
    impl_lua_event_handler, impl_lua_event_source, lua_eq, lua_fromlua_userdataonly,
    lua::{
        api::{
            utils::{LuaPoint, LuaSize},
            wm_outputs::WmOutputHandle,
        },
        event_handler::LuaEventHandler,
    },
    lua_typedef, with_hazel,
};

#[derive(Default)]
pub struct WmLayers {
    pub events: LuaEventHandler,
}

impl_lua_event_source!(WmLayers);

impl UserData for WmLayers {
    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        impl_lua_event_handler!(methods);

        methods.add_method("count", |_, _, _: ()| {
            GlobalHazel::try_with(|hazel| Ok(hazel.compositor.space.outputs().flat_map(|o| {
                let map = layer_map_for_output(o);
                map.layers().cloned().collect::<Vec<_>>()
            }).count()))
        });

        methods.add_method("get", |_, _, index: usize| {
            GlobalHazel::try_with(|hazel| {
                let idx = index.checked_sub(1);
                Ok(idx.and_then(|i| {
                    hazel.compositor.space.outputs().flat_map(|o| {
                        let map = layer_map_for_output(o);
                        map.layers().cloned().collect::<Vec<_>>()
                    }).nth(i).map(LayerHandle)
                }))
            })
        });

        methods.add_method("all", |_, _, _: ()| {
            GlobalHazel::try_with(|hazel| {
                Ok(hazel.compositor.space.outputs().flat_map(|o| {
                    let map = layer_map_for_output(o);
                    map.layers().cloned().map(LayerHandle).collect::<Vec<_>>()
                }).collect::<Vec<_>>())
            })
        });

        methods.add_method("namespace", |_, _, ns: String| {
            GlobalHazel::try_with(|hazel| {
                Ok(hazel.compositor.space.outputs().flat_map(|o| {
                    let map = layer_map_for_output(o);
                    map.layers().filter(|l| l.namespace() == ns).cloned().map(LayerHandle).collect::<Vec<_>>()
                }).collect::<Vec<_>>())
            })
        });

        methods.add_method("depth", |_, _, depth: String| {
            GlobalHazel::try_with(|hazel| {
                let target = match depth.as_str() {
                    "background" => Some(LayerDepth::Background),
                    "bottom" => Some(LayerDepth::Bottom),
                    "top" => Some(LayerDepth::Top),
                    "overlay" => Some(LayerDepth::Overlay),
                    _ => None,
                };
                Ok(hazel.compositor.space.outputs().flat_map(|o| {
                    let map = layer_map_for_output(o);
                    map.layers().filter(move |ls| Some(ls.layer()) == target).cloned().map(LayerHandle).collect::<Vec<_>>()
                }).collect::<Vec<_>>())
            })
        });

        methods.add_method("on_output", |_, _, name: String| {
            GlobalHazel::try_with(|hazel| {
                let out = hazel.compositor.space.outputs().find(|o| o.name() == name).cloned();
                let Some(out) = out else { return Ok(Vec::new()); };
                let map = layer_map_for_output(&out);
                Ok(map.layers().cloned().map(LayerHandle).collect::<Vec<_>>())
            })
        });

        methods.add_method("under", |_, _, point: LuaPoint<f64, _>| {
            GlobalHazel::try_with(|hazel| {
                for output in hazel.compositor.space.outputs() {
                    let geo = hazel.compositor.space.output_geometry(output);
                    let Some(geo) = geo else { continue; };
                    let local = point.0 - geo.loc.to_f64();
                    if local.x < 0.0 || local.y < 0.0
                        || local.x > geo.size.w as f64
                        || local.y > geo.size.h as f64
                    {
                        continue;
                    }
                    let map = layer_map_for_output(output);
                    for depth in &[LayerDepth::Overlay, LayerDepth::Top, LayerDepth::Bottom, LayerDepth::Background] {
                        if let Some(surface) = map.layer_under(*depth, local) {
                            return Ok(Some(LayerHandle(surface.clone())));
                        }
                    }
                }
                Ok(None)
            })
        });
    }
}

/// A handle to a layer surface, exposed to Lua as `Layer`.
#[derive(Clone, PartialEq)]
pub struct LayerHandle(pub DesktopLayerSurface);

lua_fromlua_userdataonly!(LayerHandle);

impl LayerHandle {
    pub fn namespace(&self) -> String {
        self.0.namespace().to_owned()
    }

    pub fn layer_name(&self) -> String {
        match self.0.layer() {
            LayerDepth::Background => "background",
            LayerDepth::Bottom => "bottom",
            LayerDepth::Top => "top",
            LayerDepth::Overlay => "overlay",
        }
        .to_owned()
    }

    pub fn position(&self) -> mlua::Result<LuaPoint<i32, Logical>> {
        with_hazel!(hazel => {
            for output in hazel.compositor.space.outputs() {
                let map = layer_map_for_output(output);
                if let Some(geo) = map.layer_geometry(&self.0) {
                    return LuaPoint(geo.loc);
                }
            }
            LuaPoint(Default::default())
        })
    }

    pub fn size(&self) -> mlua::Result<LuaSize<i32, Logical>> {
        with_hazel!(hazel => {
            for output in hazel.compositor.space.outputs() {
                let map = layer_map_for_output(output);
                if let Some(geo) = map.layer_geometry(&self.0) {
                    return LuaSize(geo.size);
                }
            }
            LuaSize(Default::default())
        })
    }

    pub fn output(&self) -> Option<WmOutputHandle> {
        let output = self.0.user_data().get::<LayerOutput>()?.0.upgrade()?;
        Some(WmOutputHandle::new(&output))
    }

    pub fn keyboard_interactivity(&self) -> &'static str {
        match self.0.cached_state().keyboard_interactivity {
            KeyboardInteractivity::None => "none",
            KeyboardInteractivity::Exclusive => "exclusive",
            KeyboardInteractivity::OnDemand => "on_demand",
        }
    }

    pub fn anchor(&self, lua: &Lua) -> mlua::Result<Table> {
        let anchor = LayerProperties::of(&self.0).anchor;
        let table = lua.create_table()?;
        table.set("top", anchor.contains(Anchor::TOP))?;
        table.set("bottom", anchor.contains(Anchor::BOTTOM))?;
        table.set("left", anchor.contains(Anchor::LEFT))?;
        table.set("right", anchor.contains(Anchor::RIGHT))?;
        Ok(table)
    }

    pub fn margin(&self, lua: &Lua) -> mlua::Result<Table> {
        let properties = LayerProperties::of(&self.0);
        let table = lua.create_table()?;
        table.set("top", properties.margin_top)?;
        table.set("right", properties.margin_right)?;
        table.set("bottom", properties.margin_bottom)?;
        table.set("left", properties.margin_left)?;
        Ok(table)
    }

    pub fn close(&self) {
        self.0.layer_surface().send_close();
    }
}

impl UserData for LayerHandle {
    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("namespace", |_, this, _: ()| Ok(this.namespace()));
        methods.add_method("layer", |_, this, _: ()| Ok(this.layer_name()));
        methods.add_method("position", |_, this, _: ()| this.position());
        methods.add_method("size", |_, this, _: ()| this.size());
        methods.add_method("output", |_, this, _: ()| Ok(this.output()));
        methods.add_method("kind", |_, _, _: ()| Ok("layer"));
        methods.add_method("keyboard_interactivity", |_, this, _: ()| {
            Ok(this.keyboard_interactivity())
        });
        methods.add_method("anchor", |lua, this, _: ()| this.anchor(lua));
        methods.add_method("margin", |lua, this, _: ()| this.margin(lua));
        methods.add_method("exclusive_zone", |_, this, _: ()| Ok(LayerProperties::of(&this.0).exclusive_zone));
        methods.add_method("exclusive_edge", |_, this, _: ()| {
            Ok(LayerProperties::of(&this.0).exclusive_edge.and_then(edge_name))
        });
        methods.add_method("requested_size", |_, this, _: ()| {
            let properties = LayerProperties::of(&this.0);
            Ok(LuaSize::<i32, Logical>((properties.requested_width, properties.requested_height).into()))
        });
        methods.add_method("close", |_, this, _: ()| {
            this.close();
            Ok(())
        });

        lua_eq!(methods);
    }
}

lua_typedef!(WmLayers => WmLayers {
    use new_surface => Layer;
    use mapped => Layer;
    use changed => Layer;
    use remove_surface => Layer;
    fn count() -> number;
    fn get(index: number) -> Option<Layer>;
    fn all() -> Array<Layer>;
    fn namespace(name: string) -> Array<Layer>;
    fn depth(depth: string) -> Array<Layer>;
    fn on_output(name: string) -> Array<Layer>;
    fn under(point: Point) -> Option<Layer>;
});

lua_typedef!(Layer => LayerHandle {
    fn namespace() -> string;
    fn layer() -> string;
    fn position() -> Point;
    fn size() -> Size;
    fn output() -> Option<WmOutput>;
    fn kind() -> string;
    fn keyboard_interactivity() -> string;
    fn anchor() -> LayerAnchor;
    fn margin() -> LayerMargin;
    fn exclusive_zone() -> integer;
    fn exclusive_edge() -> Option<string>;
    fn requested_size() -> Size;
    fn close() -> nil;
});
