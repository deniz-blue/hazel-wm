use crate::lua::{
    api::{
        wm::Wm,
        wm_input::WmInput,
        wm_input_pointer::{PointerButtonEvent, PointerMoveEvent, WmInputPointer},
        wm_input_sym::{LuaKeys, LuaKeysym, LuaMouseButton, LuaMouseButtons},
        wm_outputs::{LuaOutputMode, WmOutputHandle, WmOutputs},
		wm_input_keyboard::{WmInputKeyboard, KeyboardEvent}
    },
    typedefs::LuaTypeDef,
};

pub fn dump_typedefs() {
    println!("---@alias Array<T> T[]");
    println!("---@alias Nillable<T> T?");

    Wm::dump();

    WmInput::dump();
    WmOutputs::dump();

    WmInputPointer::dump();
    LuaMouseButton::dump();
    LuaMouseButtons::dump();
    PointerButtonEvent::dump();
    PointerMoveEvent::dump();

    LuaKeysym::dump();
    LuaKeys::dump();
    WmInputKeyboard::dump();
    KeyboardEvent::dump();

    LuaOutputMode::dump();
    WmOutputHandle::dump();
}
