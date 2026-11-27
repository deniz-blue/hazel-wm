use std::{cell::RefCell, collections::HashMap};

use mlua::{Function, Lua, RegistryKey, UserData, UserDataRef};
use tracing::error;

use crate::{
    core::{
        GlobalHazel,
        clipboard::{ClipboardChange, ClipboardOffer, FinishedRead},
    },
    impl_lua_event_handler, impl_lua_event_source,
    lua::{api::wm_input_seats::WmSeat, event_handler::LuaEventHandler},
    lua_typedef,
};

#[derive(Default)]
pub struct WmClipboard {
    pub events: LuaEventHandler,
    pending_reads: RefCell<HashMap<u64, RegistryKey>>,
}

impl_lua_event_source!(WmClipboard);

impl WmClipboard {
    pub fn deliver_changes(&self, lua: &Lua, changes: Vec<ClipboardChange>) {
        for change in changes {
            let Some(seat) = seat_named(&change.seat) else { continue };
            let offer = change.offer.map(WmClipboardOffer);
            if let Err(e) = self.events.emit_with(lua, String::from("changed"), (offer, seat)) {
                error!("Clipboard change could not reach Lua: {e}");
            }
        }
    }

    pub fn deliver_reads(&self, lua: &Lua, reads: Vec<FinishedRead>) {
        for read in reads {
            let Some(key) = self.pending_reads.borrow_mut().remove(&read.request) else {
                continue;
            };
            let callback = lua.registry_value::<Function>(&key);
            let _ = lua.remove_registry_value(key);
            let Ok(callback) = callback else { continue };
            let called = match read.result {
                Ok(bytes) => lua.create_string(&bytes).and_then(|data| callback.call::<()>((data, mlua::Nil))),
                Err(reason) => callback.call::<()>((mlua::Nil, reason)),
            };
            if let Err(e) = called {
                error!("Error in clipboard read callback: {e}");
            }
        }
    }

    fn start_read(&self, lua: &Lua, offer: u64, mime_type: &str, limit: usize, callback: Function) -> mlua::Result<()> {
        let key = lua.create_registry_value(callback)?;
        GlobalHazel::try_with(|hazel| {
            let request = hazel.compositor.clipboard.next_id();
            self.pending_reads.borrow_mut().insert(request, key);
            hazel.start_clipboard_read(request, offer, mime_type, limit);
            Ok(())
        })
    }
}

fn seat_named(name: &str) -> Option<WmSeat> {
    GlobalHazel::try_with(|hazel| Ok(hazel.compositor.seats.get(name).cloned().map(WmSeat)))
        .ok()
        .flatten()
}

fn contents_from_table(table: mlua::Table) -> mlua::Result<Vec<(String, Vec<u8>)>> {
    let mut entries = table
        .pairs::<String, mlua::String>()
        .map(|pair| pair.map(|(mime_type, data)| (mime_type, data.as_bytes().to_vec())))
        .collect::<mlua::Result<Vec<_>>>()?;
    if entries.iter().any(|(mime_type, _)| mime_type.is_empty()) {
        return Err(mlua::Error::external("a mime type must not be empty"));
    }
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(entries)
}

impl UserData for WmClipboard {
    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        impl_lua_event_handler!(methods);

        methods.add_method("current", |_, _, seat: UserDataRef<WmSeat>| {
            GlobalHazel::try_with(|hazel| {
                Ok(hazel.compositor.clipboard.current(seat.0.name()).cloned().map(WmClipboardOffer))
            })
        });

        methods.add_method("set", |_, _, (seat, contents): (UserDataRef<WmSeat>, mlua::Table)| {
            let entries = contents_from_table(contents)?;
            GlobalHazel::try_with(|hazel| {
                hazel.set_clipboard_contents(&seat.0, entries);
                Ok(())
            })
        });
    }
}

lua_typedef!(WmClipboard => WmClipboard {
    event changed(offer: Option<ClipboardOffer>, seat: Seat);
    fn current(seat: Seat) -> Option<ClipboardOffer>;
    fn set(seat: Seat, contents: table) -> nil;
});

#[derive(Clone)]
pub struct WmClipboardOffer(pub ClipboardOffer);

impl UserData for WmClipboardOffer {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("origin", |_, this| Ok(this.0.origin.name()));
        fields.add_field_method_get("seat", |_, this| Ok(seat_named(&this.0.seat)));
    }

    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("mime_types", |_, this, ()| Ok(this.0.mime_types.clone()));

        methods.add_method("is_current", |_, this, ()| {
            GlobalHazel::try_with(|hazel| {
                Ok(hazel.compositor.clipboard.current(&this.0.seat).is_some_and(|current| current.id == this.0.id))
            })
        });

        methods.add_method("read", |lua, this, (mime_type, max_bytes, callback): (String, usize, Function)| {
            let clipboard = GlobalHazel::try_with(|hazel| Ok(hazel.wm().clipboard.clone()))?;
            clipboard.start_read(lua, this.0.id, &mime_type, max_bytes, callback)
        });
    }
}

lua_typedef!(ClipboardOffer => WmClipboardOffer {
    let origin: string;
    let seat: Option<Seat>;
    fn mime_types() -> Array<string>;
    fn is_current() -> boolean;
    fn read(mime_type: string, max_bytes: integer, callback: function) -> nil;
});
