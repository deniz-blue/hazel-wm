#[derive(Debug, Clone)]
pub enum AppEvent {
    KeyPress {
        keycode: u32,
		serial: u32,
		time: u32,
    },
    PointerMotion {
        position: (f64, f64),
    },
    PointerButton {
        button: u32,
    },
}
