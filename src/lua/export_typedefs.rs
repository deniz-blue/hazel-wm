use smithay::utils::Logical;

use crate::lua::{
    api::{
        utils::{LuaPoint, LuaSize},
        wm::Wm,
        wm_input::WmInput,
        wm_input_keyboard::{KeyEvent, ModifiersStateUserData, WmInputKeyboard},
        wm_input_pointer::{PointerButtonEvent, PointerMoveEvent, WmInputPointer},
        wm_input_sym::{LuaKeys, LuaKeysym, LuaMouseButton, LuaMouseButtons},
        wm_outputs::{LuaOutputMode, WmOutputHandle, WmOutputs},
        wm_windows::{WmWindow, WmWindows},
    },
    typedefs::LuaTypeDef,
};

pub fn dump_typedefs() {
    println!("---@meta");
    println!();
    println!("---@alias Array<T> T[]");
    println!("---@alias Option<T> T?");
    println!();
    println!("---@type table<string, Keysym>");
    println!("Key = {{}}");
    println!();
    println!("---@type table<string, MouseButton>");
    println!("Button = {{}}");
    println!();

    LuaPoint::<f64, Logical>::dump();
    LuaSize::<f64, Logical>::dump();

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
    KeyEvent::dump();
    ModifiersStateUserData::dump();

    LuaOutputMode::dump();
    WmOutputHandle::dump();

    WmWindows::dump();
    WmWindow::dump();

    println!("---@param command string");
    println!("function spawn(command) end");
}
