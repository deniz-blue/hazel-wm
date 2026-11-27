use std::{os::fd::OwnedFd, sync::Arc};

use smithay::{
    input::Seat,
    wayland::selection::{
        SelectionHandler, SelectionSource, SelectionTarget,
        primary_selection::{PrimarySelectionHandler, PrimarySelectionState},
    },
};

use tracing::warn;

use crate::core::{
    Hazel, fd_transfer,
    clipboard::{OfferOrigin, StoredContents},
};

impl SelectionHandler for Hazel {
    type SelectionUserData = Arc<StoredContents>;

    fn new_selection(&mut self, ty: SelectionTarget, source: Option<SelectionSource>, seat: Seat<Self>) {
        let name = seat.name();
        if ty == SelectionTarget::Clipboard && !self.compositor.clipboard.publishing(name) {
            match source {
                Some(source) => self.compositor.clipboard.offer(name, OfferOrigin::Wayland, source.mime_types()),
                None => self.compositor.clipboard.clear(name),
            }
        }
    }

    fn send_selection(&mut self, _: SelectionTarget, mime_type: String, fd: OwnedFd, _: Seat<Self>, contents: &Arc<StoredContents>) {
        let handle = self.compositor.loop_handle.clone();
        if let Some(bytes) = contents.bytes_for(&mime_type)
            && let Err(e) = fd_transfer::write_all(&handle, fd, bytes)
        {
            warn!("Could not hand out the stored clipboard: {e}");
        }
    }
}

impl PrimarySelectionHandler for Hazel {
    fn primary_selection_state(&mut self) -> &mut PrimarySelectionState {
        &mut self.compositor.smithay.primary_selection_state
    }
}

