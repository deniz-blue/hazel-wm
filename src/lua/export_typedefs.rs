use smithay::utils::Logical;

use crate::lua::{
    api::{
        utils::{LuaPoint, LuaRect, LuaSize},
        wm::Wm,
        wm_input::WmInput,
        wm_input_keyboard::{KeyEvent, ModifiersStateUserData, WmInputKeyboard},
        wm_input_pointer::{PointerButtonEvent, PointerMoveEvent, WmInputPointer},
        wm_input_seats::{WmInputSeats, WmSeat},
        wm_input_sym::{LuaKeys, LuaKeysym, LuaMouseButton, LuaMouseButtons},
        wm_layers::{LayerHandle, WmLayers},
        wm_outputs::{LuaOutputMode, WmOutputHandle, WmOutputs},
        wm_windows::{WindowStates, WmWindow, WmWindows},
    },
    typedefs::LuaTypeDef,
};

pub fn dump_typedefs() {
    println!("---@meta");
    println!();
    println!("-- GENERATED FILE, DO NOT EDIT MANUALLY");
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
    println!("Point = Point\n");
    LuaSize::<f64, Logical>::dump();
    LuaRect::dump();
    println!("Rect = Rect\n");

    Wm::dump();

    WmInput::dump();
    WmInputSeats::dump();
    WmOutputs::dump();

    WmSeat::dump();
    WmInputPointer::dump();
    // Manual: "move" is a Rust keyword, can't use in macro
    println!("--- @class Pointer");
    println!("--- @param point Point");
    println!("--- @return nil");
    println!("function Pointer:move(point) end\n");
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
    WindowStates::dump();

    WmLayers::dump();
    LayerHandle::dump();

    println!("---@param command string");
    println!("function spawn(command) end");
}
