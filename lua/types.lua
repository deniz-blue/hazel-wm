---@meta

---@alias Array<T> T[]
---@alias Option<T> T?

---@type table<string, Keysym>
Key = {}

---@type table<string, MouseButton>
Button = {}

--- @class Point
--- @field x number
--- @field y number
--- @param new Point
--- @return Point
function Point(new) end

--- @class Point
--- @param x number
--- @param y number
--- @return Point
function Point(x, y) end

--- @class Point
--- @param other Point
--- @return nil
function Point:add(other) end

--- @class Size
--- @field width number
--- @field height number
local Size = {}

--- @class Wm
--- @field name string
--- @field input WmInput
--- @field windows WmWindows
--- @field outputs WmOutputs
local Wm = {}

wm = Wm
--- @return nil
function Wm:quit() end

--- @param event "ready"
--- @param callback fun(e: nil)
function Wm:on(event, callback) end

--- @param event "tick"
--- @param callback fun(e: nil)
function Wm:on(event, callback) end

--- @class WmInput
--- @field seats WmSeats
local WmInput = {}

--- @param event "key"
--- @param callback fun(e: KeyEvent)
function WmInput:on(event, callback) end

--- @param event "pointer_move"
--- @param callback fun(e: PointerMoveEvent)
function WmInput:on(event, callback) end

--- @param event "pointer_button"
--- @param callback fun(e: PointerButtonEvent)
function WmInput:on(event, callback) end

--- @param event "new_keyboard"
--- @param callback fun(e: Keyboard)
function WmInput:on(event, callback) end

--- @param event "new_pointer"
--- @param callback fun(e: Pointer)
function WmInput:on(event, callback) end

--- @class WmSeats
local WmSeats = {}

--- @return integer
function WmSeats:count() end

--- @param name string
--- @return Option<Seat>
function WmSeats:get(name) end

--- @class WmOutputs
local WmOutputs = {}

--- @return number
function WmOutputs:count() end

--- @param name string
--- @return WmOutput
function WmOutputs:name(name) end

--- @class Seat
--- @field name string
local Seat = {}

--- @return Option<Pointer>
function Seat:pointer() end

--- @return Option<Keyboard>
function Seat:keyboard() end

--- @class Pointer
local Pointer = {}

--- @return Seat
function Pointer:seat() end

--- @return Point
function Pointer:position() end

--- @return table<MouseButton>
function Pointer:buttons() end

--- @return Option<Window>
function Pointer:window_under() end

--- @class MouseButton
local MouseButton = {}

--- @class MouseButtons
local MouseButtons = {}

Button = MouseButtons
--- @class PointerButtonEvent
--- @field button MouseButton
--- @field state string
--- @field pointer Pointer
local PointerButtonEvent = {}

--- @return nil
function PointerButtonEvent:prevent_default() end

--- @class PointerMoveEvent
--- @field delta Point
--- @field delta_unaccel Point
--- @field position Point
--- @field output_position Option<Point>
--- @field pointer Pointer
local PointerMoveEvent = {}

--- @return nil
function PointerMoveEvent:prevent_default() end

--- @class Keysym
local Keysym = {}

--- @class Keys
local Keys = {}

Key = Keys
--- @class Keyboard
local Keyboard = {}

--- @return Seat
function Keyboard:seat() end

--- @return string
function Keyboard:get_layout() end

--- @param keymap string
--- @return boolean
function Keyboard:set_layout(keymap) end

--- @class KeyEvent
--- @field keyboard Keyboard
--- @field state string
--- @field serial number
--- @field time number
--- @field keycode number
--- @field key Keysym
--- @field keys Array<Keysym>
--- @field modifiers Modifiers
local KeyEvent = {}

--- @return nil
function KeyEvent:prevent_default() end

--- @class Modifiers
--- @field shift boolean
--- @field ctrl boolean
--- @field alt boolean
--- @field logo boolean
--- @field caps_lock boolean
--- @field num_lock boolean
--- @field altgr boolean
local Modifiers = {}

--- @class OutputMode
--- @field size Size
--- @field refresh number
local OutputMode = {}

--- @class WmOutput
--- @field name string
--- @field description string
--- @field properties table
local WmOutput = {}

--- @return OutputMode
function WmOutput:mode() end

--- @return Point
function WmOutput:position() end

--- @param point Point
--- @return nil
function WmOutput:set_position(point) end

--- @return Size
function WmOutput:size() end

--- @class WmWindows
local WmWindows = {}

--- @param event "new_window"
--- @param callback fun(e: Window)
function WmWindows:on(event, callback) end

--- @class Window
local Window = {}

--- @return Point
function Window:position() end

--- @param position Point
--- @return nil
function Window:set_position(position) end

--- @return Size
function Window:size() end

--- @param size Size
--- @return nil
function Window:set_size(size) end

---@param command string
function spawn(command) end
