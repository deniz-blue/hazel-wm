use smithay::utils::Logical;

use crate::lua::{
    api::{
        utils::{LuaPoint, LuaRect, LuaSize},
        wm::Wm,
        wm_clipboard::{WmClipboard, WmClipboardOffer},
        wm_input::WmInput,
        wm_input_keyboard::{KeyEvent, ModifiersStateUserData, WmInputKeyboard},
        wm_window_requests::WindowDragRequest,
        wm_input_constraints::WmPointerConstraint,
        wm_input_gestures::PointerGestureEvent,
        wm_input_pointer::{PointerButtonEvent, PointerMoveEvent, WmInputPointer},
        wm_input_seats::{WmInputSeats, WmSeat},
        wm_input_shortcuts::WmShortcutsInhibitor,
        wm_input_sym::{LuaKeys, LuaKeysym, LuaMouseButton, LuaMouseButtons},
        wm_layers::{LayerHandle, WmLayers},
        wm_output_modes::LuaOutputMode,
        wm_session_lock::{WmLockSurface, WmSessionLock},
        wm_shapes::{WmShape, WmText},
        wm_outputs::{WmOutputHandle, WmOutputs},
        wm_timers::WmTimer,
        wm_window_states::WindowStates,
        wm_windows::{WmWindow, WmWindows},
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
    WmTimer::dump();

    println!("--- @alias FocusTarget Window|Layer\n");
    WmInput::dump();
    WmInputSeats::dump();
    WmShortcutsInhibitor::dump();
    WmOutputs::dump();
    WmClipboard::dump();
    WmSessionLock::dump();
    WmLockSurface::dump();
    WmClipboardOffer::dump();

    WmSeat::dump();
    WmInputPointer::dump();
    // Manual: "move" is a Rust keyword, can't use in macro
    println!("--- @class Pointer");
    println!("--- @param point Point");
    println!("--- @return nil");
    println!("function Pointer:move(point) end\n");
    println!("--- @class Pointer");
    println!("--- @param button MouseButton");
    println!("--- @param state \"pressed\"|\"released\"");
    println!("--- @return nil");
    println!("function Pointer:button(button, state) end\n");
    println!("--- @class Keyboard");
    println!("--- @param evdev_code integer linux input-event-codes.h value, e.g. 30 for A");
    println!("--- @param state \"pressed\"|\"released\"");
    println!("--- @return nil");
    println!("function Keyboard:key(evdev_code, state) end\n");
    LuaMouseButton::dump();
    LuaMouseButtons::dump();
    PointerButtonEvent::dump();
    PointerMoveEvent::dump();
    PointerGestureEvent::dump();
    WmPointerConstraint::dump();

    LuaKeysym::dump();
    LuaKeys::dump();
    WmInputKeyboard::dump();
    KeyEvent::dump();
    ModifiersStateUserData::dump();

    LuaOutputMode::dump();
    WmOutputHandle::dump();

    WmWindows::dump();
    println!("--- @param event \"activation_request\"");
    println!("--- @param callback fun(window: Window, token_age_ms: integer)");
    println!("function WmWindows:on(event, callback) end\n");
    println!("--- @param event \"fullscreen_request\"");
    println!("--- @param callback fun(window: Window, output: WmOutput?)");
    println!("function WmWindows:on(event, callback) end\n");
    println!("--- @param event \"bell\"");
    println!("--- @param callback fun(window: Window?)");
    println!("function WmWindows:on(event, callback) end\n");
    WmWindow::dump();
    WindowStates::dump();
    WindowDragRequest::dump();

    WmLayers::dump();
    LayerHandle::dump();

    WmShape::dump();
    WmText::dump();
    for line in [
        "---@class FontSpec",
        "---@field family string? a family name, or \"sans-serif\", \"serif\", \"monospace\"; the default is \"sans-serif\"",
        "---@field file string? path of a font file, instead of a family",
        "---@field size number? logical pixels, above 0 and at most 1024; the default is 14",
        "---@field weight integer? 1 to 1000; the default is 400",
        "",
        "---@class TextMetrics",
        "---@field width number",
        "---@field height number",
        "---@field baseline number distance from the top to the baseline",
        "---@field text string what would be drawn: line breaks turned into spaces, and cut with an ellipsis to max_width",
        "",
        "---@class LayerAnchor",
        "---@field top boolean",
        "---@field bottom boolean",
        "---@field left boolean",
        "---@field right boolean",
        "",
        "---@class LayerMargin",
        "---@field top integer",
        "---@field right integer",
        "---@field bottom integer",
        "---@field left integer",
        "",
        "---@alias ShapeLayer \"background\"|\"bottom\"|\"top\"|\"overlay\"",
        "",
        "---@class ShapeSpec",
        "---@field kind \"rect\"|\"text\" required when creating, not allowed in update",
        "---@field x integer? left edge, relative to the window's top left or the output's top left",
        "---@field y integer?",
        "---@field z integer? higher is in front; on a window, below 0 is behind the window's surface",
        "---@field layer ShapeLayer? outputs only; the default is \"top\"",
        "---@field visible boolean?",
        "---@field width integer? rect only",
        "---@field height integer? rect only",
        "---@field fill string? rect only, \"#rrggbb[aa]\", or false for none",
        "---@field radius integer? rect only, corner radius",
        "---@field border_width integer? rect only, drawn inside the rectangle",
        "---@field border_color string? rect only",
        "---@field text string? text only",
        "---@field font FontSpec? text only; fields not given keep their value",
        "---@field color string? text only",
        "---@field max_width integer|false? text only; longer text is cut with an ellipsis",
        "",
    ] {
        println!("{line}");
    }

    for line in [
        "---@class ExitReport",
        "---@field status integer? exit code, nil when the process was killed by a signal",
        "---@field signal integer? the signal that killed the process",
        "---@field timed_out boolean",
        "",
        "---@class ExecResult : ExitReport",
        "---@field stdout string",
        "---@field stderr string",
        "",
        "---@class ProcessOptions",
        "---@field env table<string, string>?",
        "---@field cwd string?",
        "---@field timeout number? milliseconds before the process gets SIGTERM, and SIGKILL a second later",
        "---@field on_stdout fun(line: string)?",
        "---@field on_stderr fun(line: string)?",
        "---@field on_exit fun(report: ExitReport)?",
        "---@field survives_reload boolean? keep the process running through a reload; its callbacks are dropped",
        "",
        "--- Starts a process without waiting for it. Processes with an on_* callback are stopped on reload unless survives_reload is set.",
        "---@param program string",
        "---@param args string[]?",
        "---@param options ProcessOptions?",
        "---@return integer pid",
        "function spawn(program, args, options) end",
        "",
        "--- Runs a process to completion, blocking the compositor meanwhile. Use timeout to bound that.",
        "---@param program string",
        "---@param args string[]?",
        "---@param options ProcessOptions?",
        "---@return ExecResult",
        "function exec(program, args, options) end",
        "",
        "--- Signals a process started with spawn.",
        "---@param pid integer",
        "---@param signal \"term\"|\"kill\"|\"int\"|\"hup\"?",
        "---@return boolean found",
        "function kill(pid, signal) end",
    ] {
        println!("{line}");
    }
}
