use std::{io::pipe, os::fd::OwnedFd, sync::Arc};

use smithay::{
    input::Seat,
    wayland::selection::data_device::{
        SelectionRequestError, clear_data_device_selection, request_data_device_client_selection,
        set_data_device_selection,
    },
};

use crate::core::{
    Hazel, fd_transfer,
    clipboard::{ClipboardOffer, OfferOrigin, StoredContents},
};

impl Hazel {
    pub fn start_clipboard_read(&mut self, request: u64, offer: u64, mime_type: &str, limit: usize) {
        let result = self.begin_clipboard_read(request, offer, mime_type, limit);
        if let Err(reason) = result {
            self.compositor.clipboard.finish_read(request, offer, Err(reason));
        }
    }

    fn begin_clipboard_read(&mut self, request: u64, offer: u64, mime_type: &str, limit: usize) -> Result<(), String> {
        let current = self.compositor.clipboard.current_with_id(offer).cloned();
        let Some(current) = current else {
            return Err(String::from("the clipboard has changed"));
        };
        if !current.mime_types.iter().any(|offered| offered == mime_type) {
            return Err(format!("{mime_type} is not offered"));
        }
        match current.origin {
            OfferOrigin::Lua => self.read_stored_clipboard(request, &current, mime_type, limit),
            OfferOrigin::Wayland => self.read_client_clipboard(request, &current, mime_type, limit),
        }
    }

    fn read_stored_clipboard(&mut self, request: u64, offer: &ClipboardOffer, mime_type: &str, limit: usize) -> Result<(), String> {
        let bytes = self
            .compositor
            .clipboard
            .stored(&offer.seat)
            .and_then(|stored| stored.bytes_for(mime_type))
            .ok_or_else(|| format!("{mime_type} is not stored"))?;
        let result = if bytes.len() > limit { Err(format!("more than {limit} bytes")) } else { Ok(bytes.to_vec()) };
        self.compositor.clipboard.finish_read(request, offer.id, result);
        Ok(())
    }

    fn read_client_clipboard(&mut self, request: u64, offer: &ClipboardOffer, mime_type: &str, limit: usize) -> Result<(), String> {
        let seat = self.compositor.seats.get(&offer.seat).cloned().ok_or("the seat is gone")?;
        let handle = self.compositor.loop_handle.clone();
        let offer_id = offer.id;
        let (reader, writer) = pipe().map_err(|e| e.to_string())?;
        fd_transfer::read_limited(&handle, reader.into(), limit, move |hazel, result| {
            hazel.compositor.clipboard.finish_read(request, offer_id, result.map_err(|e| e.to_string()));
        })
        .map_err(|e| e.to_string())?;
        request_from_wayland_client(&seat, mime_type, writer.into())
    }

    pub fn set_clipboard_contents(&mut self, seat: &Seat<Hazel>, entries: Vec<(String, Vec<u8>)>) {
        let name = seat.name().to_owned();
        let contents = (!entries.is_empty()).then(|| Arc::new(StoredContents::new(entries)));
        self.compositor.clipboard.set_publishing(&name, true);
        match &contents {
            Some(contents) => set_data_device_selection(&self.display_handle, seat, contents.mime_types(), contents.clone()),
            None => clear_data_device_selection(&self.display_handle, seat),
        }
        self.compositor.clipboard.set_publishing(&name, false);
        self.compositor.clipboard.store(&name, contents.clone());
        match contents {
            Some(contents) => self.compositor.clipboard.offer(&name, OfferOrigin::Lua, contents.mime_types()),
            None => self.compositor.clipboard.clear(&name),
        }
    }
}

fn request_from_wayland_client(seat: &Seat<Hazel>, mime_type: &str, fd: OwnedFd) -> Result<(), String> {
    request_data_device_client_selection(seat, mime_type.to_owned(), fd).map_err(|e: SelectionRequestError| format!("{e:?}"))
}
