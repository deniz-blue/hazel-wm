use std::{collections::HashMap, sync::Arc};

#[derive(Debug, Default)]
pub struct StoredContents {
    entries: Vec<(String, Arc<[u8]>)>,
}

impl StoredContents {
    pub fn new(entries: Vec<(String, Vec<u8>)>) -> Self {
        Self {
            entries: entries
                .into_iter()
                .map(|(mime_type, bytes)| (mime_type, Arc::from(bytes)))
                .collect(),
        }
    }

    pub fn mime_types(&self) -> Vec<String> {
        self.entries.iter().map(|(mime_type, _)| mime_type.clone()).collect()
    }

    pub fn bytes_for(&self, mime_type: &str) -> Option<Arc<[u8]>> {
        self.entries
            .iter()
            .find(|(candidate, _)| candidate == mime_type)
            .map(|(_, bytes)| bytes.clone())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfferOrigin {
    Wayland,
    Lua,
}

impl OfferOrigin {
    pub fn name(self) -> &'static str {
        match self {
            Self::Wayland => "wayland",
            Self::Lua => "lua",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClipboardOffer {
    pub id: u64,
    pub seat: String,
    pub origin: OfferOrigin,
    pub mime_types: Vec<String>,
}

#[derive(Debug)]
pub struct ClipboardChange {
    pub seat: String,
    pub offer: Option<ClipboardOffer>,
}

#[derive(Debug)]
pub struct FinishedRead {
    pub request: u64,
    pub offer: u64,
    pub result: Result<Vec<u8>, String>,
}

#[derive(Debug, Default)]
struct SeatClipboard {
    current: Option<ClipboardOffer>,
    stored: Option<Arc<StoredContents>>,
    publishing: bool,
}

#[derive(Debug, Default)]
pub struct ClipboardState {
    next_id: u64,
    seats: HashMap<String, SeatClipboard>,
    changes: Vec<ClipboardChange>,
    finished_reads: Vec<FinishedRead>,
}

impl ClipboardState {
    pub fn next_id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }

    fn seat_mut(&mut self, seat: &str) -> &mut SeatClipboard {
        self.seats.entry(seat.to_owned()).or_default()
    }

    pub fn current(&self, seat: &str) -> Option<&ClipboardOffer> {
        self.seats.get(seat)?.current.as_ref()
    }

    pub fn current_with_id(&self, offer: u64) -> Option<&ClipboardOffer> {
        self.seats
            .values()
            .filter_map(|clipboard| clipboard.current.as_ref())
            .find(|current| current.id == offer)
    }

    pub fn offer(&mut self, seat: &str, origin: OfferOrigin, mime_types: Vec<String>) {
        let offer = ClipboardOffer { id: self.next_id(), seat: seat.to_owned(), origin, mime_types };
        self.seat_mut(seat).current = Some(offer.clone());
        self.changes.push(ClipboardChange { seat: seat.to_owned(), offer: Some(offer) });
    }

    pub fn clear(&mut self, seat: &str) {
        self.seat_mut(seat).current = None;
        self.changes.push(ClipboardChange { seat: seat.to_owned(), offer: None });
    }

    pub fn stored(&self, seat: &str) -> Option<Arc<StoredContents>> {
        self.seats.get(seat)?.stored.clone()
    }

    pub fn store(&mut self, seat: &str, contents: Option<Arc<StoredContents>>) {
        self.seat_mut(seat).stored = contents;
    }

    pub fn publishing(&self, seat: &str) -> bool {
        self.seats.get(seat).is_some_and(|clipboard| clipboard.publishing)
    }

    pub fn set_publishing(&mut self, seat: &str, publishing: bool) {
        self.seat_mut(seat).publishing = publishing;
    }

    pub fn finish_read(&mut self, request: u64, offer: u64, result: Result<Vec<u8>, String>) {
        self.finished_reads.push(FinishedRead { request, offer, result });
    }

    pub fn take_changes(&mut self) -> Vec<ClipboardChange> {
        std::mem::take(&mut self.changes)
    }

    pub fn take_finished_reads(&mut self) -> Vec<FinishedRead> {
        std::mem::take(&mut self.finished_reads)
            .into_iter()
            .map(|read| match read.result {
                Ok(_) if self.current_with_id(read.offer).is_none() => FinishedRead {
                    result: Err(String::from("the clipboard changed while it was being read")),
                    ..read
                },
                _ => read,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn types(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn each_offer_replaces_the_current_one_of_its_seat_and_is_reported() {
        let mut state = ClipboardState::default();
        state.offer("seat0", OfferOrigin::Lua, types(&["text/plain"]));
        state.offer("seat0", OfferOrigin::Wayland, types(&["image/png"]));
        assert_eq!(state.current("seat0").map(|offer| offer.origin), Some(OfferOrigin::Wayland));
        let changes = state.take_changes();
        assert_eq!(changes.len(), 2);
        assert_ne!(changes[0].offer.as_ref().map(|o| o.id), changes[1].offer.as_ref().map(|o| o.id));
        assert!(state.take_changes().is_empty());
    }

    #[test]
    fn seats_keep_separate_clipboards() {
        let mut state = ClipboardState::default();
        state.offer("seat0", OfferOrigin::Wayland, types(&["text/plain"]));
        state.offer("seat1", OfferOrigin::Lua, types(&["image/png"]));
        assert_eq!(state.current("seat0").map(|offer| offer.mime_types.clone()), Some(types(&["text/plain"])));
        assert_eq!(state.current("seat1").map(|offer| offer.origin), Some(OfferOrigin::Lua));
        state.clear("seat1");
        assert!(state.current("seat1").is_none());
        assert!(state.current("seat0").is_some());
        let changes = state.take_changes();
        assert_eq!(changes.iter().map(|change| change.seat.as_str()).collect::<Vec<_>>(), ["seat0", "seat1", "seat1"]);
    }

    #[test]
    fn clearing_is_reported_as_nothing() {
        let mut state = ClipboardState::default();
        state.offer("seat0", OfferOrigin::Wayland, types(&["text/plain"]));
        state.take_changes();
        state.clear("seat0");
        assert!(state.current("seat0").is_none());
        assert!(matches!(state.take_changes().as_slice(), [ClipboardChange { offer: None, .. }]));
    }

    #[test]
    fn a_read_of_an_older_offer_is_reported_as_failed() {
        let mut state = ClipboardState::default();
        state.offer("seat0", OfferOrigin::Wayland, types(&["text/plain"]));
        let old = state.current("seat0").map(|offer| offer.id).unwrap_or_default();
        state.finish_read(1, old, Ok(b"x".to_vec()));
        state.offer("seat0", OfferOrigin::Wayland, types(&["text/plain"]));
        let reads = state.take_finished_reads();
        assert!(reads[0].result.is_err());
    }

    #[test]
    fn a_read_of_the_current_offer_keeps_its_bytes() {
        let mut state = ClipboardState::default();
        state.offer("seat0", OfferOrigin::Wayland, types(&["text/plain"]));
        let current = state.current("seat0").map(|offer| offer.id).unwrap_or_default();
        state.finish_read(1, current, Ok(b"x".to_vec()));
        assert_eq!(state.take_finished_reads()[0].result.as_deref(), Ok(&b"x"[..]));
    }

    #[test]
    fn a_read_survives_another_seat_changing() {
        let mut state = ClipboardState::default();
        state.offer("seat0", OfferOrigin::Wayland, types(&["text/plain"]));
        let current = state.current("seat0").map(|offer| offer.id).unwrap_or_default();
        state.finish_read(1, current, Ok(b"x".to_vec()));
        state.offer("seat1", OfferOrigin::Wayland, types(&["text/plain"]));
        assert!(state.take_finished_reads()[0].result.is_ok());
    }

    #[test]
    fn stored_contents_answer_by_mime_type() {
        let contents = StoredContents::new(vec![("text/plain".into(), b"hello".to_vec())]);
        assert_eq!(contents.mime_types(), ["text/plain"]);
        assert_eq!(&*contents.bytes_for("text/plain").expect("bytes"), b"hello");
        assert!(contents.bytes_for("image/png").is_none());
    }
}
