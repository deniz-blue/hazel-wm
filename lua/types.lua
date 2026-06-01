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
local Point = {}

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

--- @class WmInput
local WmInput = {}

--- @param event "key"
--- @param callback fun(e: KeyboardEvent)
function WmInput:on(event, callback) end

--- @param event "pointer_move"
--- @param callback fun(e: PointerMoveEvent)
function WmInput:on(event, callback) end

--- @param event "pointer_button"
--- @param callback fun(e: PointerButtonEvent)
function WmInput:on(event, callback) end

--- @class WmOutputs
local WmOutputs = {}

--- @return number
function WmOutputs:count() end

--- @param name string
--- @return WmOutput
function WmOutputs:name(name) end

--- @class Pointer
local Pointer = {}

--- @return Point
function Pointer:position() end

--- @return table<MouseButton>
function Pointer:buttons() end

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

--- @return string
function Keyboard:get_layout() end

--- @param keymap string
--- @return boolean
function Keyboard:set_layout(keymap) end

--- @class KeyboardEvent
--- @field state string
--- @field serial number
--- @field time number
--- @field keycode number
--- @field key Keysym
--- @field keys Array<Keysym>
--- @field modifiers ModifiersState
local KeyboardEvent = {}

--- @return nil
function KeyboardEvent:prevent_default() end

--- @class ModifiersState
--- @field shift boolean
--- @field ctrl boolean
--- @field alt boolean
--- @field logo boolean
--- @field caps_lock boolean
--- @field num_lock boolean
--- @field altgr boolean
local ModifiersState = {}

--- @class OutputMode
--- @field size Size
--- @field refresh number
local OutputMode = {}

--- @class WmOutput
--- @field name string
--- @field description string
--- @field mode OutputMode
--- @field properties table
local WmOutput = {}

--- @return Point
function WmOutput:position() end

--- @param point Point
--- @return nil
function WmOutput:set_position(point) end

---@param command string
function spawn(command) end
