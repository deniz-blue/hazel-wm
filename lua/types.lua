---@meta

-- GENERATED FILE, DO NOT EDIT MANUALLY

---@alias Array<T> T[]
---@alias Option<T> T?

---@type table<string, Keysym>
Key = {}

---@type table<string, MouseButton>
Button = {}

--- @class Point
--- @field x number
--- @field y number
--- @overload fun(new: Point): Point
--- @overload fun(x: number, y: number): Point
local Point = {}

--- @class Point
--- @param other Point
--- @return nil
function Point:add(other) end

Point = Point

--- @class Size
--- @field width number
--- @field height number
local Size = {}

--- @class Rect
--- @field x number
--- @field y number
--- @field width number
--- @field height number
--- @overload fun(x: number, y: number, width: number, height: number): Rect
--- @overload fun(table: table): Rect
local Rect = {}

--- @class Rect
--- @param point Point
--- @return boolean
function Rect:contains(point) end

--- @class Rect
--- @param other Rect
--- @return boolean
function Rect:intersects(other) end

--- @class Rect
--- @param other Rect
--- @return Rect
function Rect:intersection(other) end

--- @class Rect
--- @param other Rect
--- @return Rect
function Rect:merge(other) end

--- @class Rect
--- @param amount number
--- @return Rect
function Rect:inset(amount) end

Rect = Rect

--- @class Wm
--- @field name string
--- @field input WmInput
--- @field windows WmWindows
--- @field layers WmLayers
--- @field outputs WmOutputs
local Wm = {}

wm = Wm
--- @class Wm
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

--- @class WmSeats
--- @return integer
function WmSeats:count() end

--- @class WmSeats
--- @param name string
--- @return Option<Seat>
function WmSeats:get(name) end

--- @class WmOutputs
local WmOutputs = {}

--- @class WmOutputs
--- @return number
function WmOutputs:count() end

--- @class WmOutputs
--- @param index number
--- @return WmOutput
function WmOutputs:get(index) end

--- @class WmOutputs
--- @return Array<WmOutput>
function WmOutputs:all() end

--- @class WmOutputs
--- @param name string
--- @return WmOutput
function WmOutputs:name(name) end

--- @class WmOutputs
--- @param point Point
--- @return Array<WmOutput>
function WmOutputs:under(point) end

--- @class Seat
--- @field name string
local Seat = {}

--- @class Seat
--- @return Option<Pointer>
function Seat:pointer() end

--- @class Seat
--- @return Option<Keyboard>
function Seat:keyboard() end

--- @class Pointer
local Pointer = {}

--- @class Pointer
--- @return Seat
function Pointer:seat() end

--- @class Pointer
--- @return Point
function Pointer:position() end

--- @class Pointer
--- @return table<MouseButton>
function Pointer:buttons() end

--- @class Pointer
--- @return Option<Window>
function Pointer:window_under() end

--- @class Pointer
--- @param point Point
--- @return nil
function Pointer:move(point) end

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

--- @class PointerButtonEvent
--- @return nil
function PointerButtonEvent:prevent_default() end

--- @class PointerMoveEvent
--- @field delta Point
--- @field delta_unaccel Point
--- @field position Point
--- @field output_position Option<Point>
--- @field pointer Pointer
local PointerMoveEvent = {}

--- @class PointerMoveEvent
--- @return nil
function PointerMoveEvent:prevent_default() end

--- @class Keysym
local Keysym = {}

--- @class Keys
local Keys = {}

Key = Keys
--- @class Keyboard
local Keyboard = {}

--- @class Keyboard
--- @return Seat
function Keyboard:seat() end

--- @class Keyboard
--- @return string
function Keyboard:get_layout() end

--- @class Keyboard
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

--- @class KeyEvent
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

--- @class WmOutput
--- @return OutputMode
function WmOutput:mode() end

--- @class WmOutput
--- @return Point
function WmOutput:position() end

--- @class WmOutput
--- @param point Point
--- @return nil
function WmOutput:set_position(point) end

--- @class WmOutput
--- @return Size
function WmOutput:size() end

--- @class WmWindows
local WmWindows = {}

--- @class WmWindows
--- @return number
function WmWindows:count() end

--- @class WmWindows
--- @param index number
--- @return Window
function WmWindows:get(index) end

--- @class WmWindows
--- @return Array<Window>
function WmWindows:all() end

--- @class WmWindows
--- @param point Point
--- @return Window
function WmWindows:under(point) end

--- @param event "new_window"
--- @param callback fun(e: Window)
function WmWindows:on(event, callback) end

--- @class Window
local Window = {}

--- @class Window
--- @return Point
function Window:position() end

--- @class Window
--- @param position Point
--- @return nil
function Window:set_position(position) end

--- @class Window
--- @return Size
function Window:size() end

--- @class Window
--- @param size Size
--- @return nil
function Window:set_size(size) end

--- @class Window
--- @return string
function Window:title() end

--- @class Window
--- @return string
function Window:app_id() end

--- @class Window
--- @return nil
function Window:close() end

--- @class Window
--- @param active boolean
--- @return nil
function Window:set_activated(active) end

--- @class Window
--- @return WindowStates
function Window:states() end

--- @class Window
--- @param name string
--- @param enabled boolean
--- @return nil
function Window:set_state(name, enabled) end

--- @class WindowStates
local WindowStates = {}

--- @class WindowStates
--- @param name string
--- @return boolean
function WindowStates:has(name) end

--- @class WindowStates
--- @return Array<string>
function WindowStates:list() end

--- @class WindowStates
--- @param name string
--- @param enabled boolean
--- @return nil
function WindowStates:set(name, enabled) end

--- @class WmLayers
local WmLayers = {}

--- @class WmLayers
--- @return number
function WmLayers:count() end

--- @class WmLayers
--- @param index number
--- @return Layer
function WmLayers:get(index) end

--- @class WmLayers
--- @return Array<Layer>
function WmLayers:all() end

--- @class WmLayers
--- @param name string
--- @return Array<Layer>
function WmLayers:namespace(name) end

--- @class WmLayers
--- @param depth string
--- @return Array<Layer>
function WmLayers:depth(depth) end

--- @class WmLayers
--- @param name string
--- @return Array<Layer>
function WmLayers:on_output(name) end

--- @class WmLayers
--- @param point Point
--- @return Layer
function WmLayers:under(point) end

--- @class Layer
local Layer = {}

--- @class Layer
--- @return string
function Layer:namespace() end

--- @class Layer
--- @return string
function Layer:layer() end

--- @class Layer
--- @return Point
function Layer:position() end

--- @class Layer
--- @return Size
function Layer:size() end

--- @class Layer
--- @return WmOutput
function Layer:output() end

--- @class Layer
--- @return nil
function Layer:close() end

---@param command string
function spawn(command) end
