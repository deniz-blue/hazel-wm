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
--- @return Option<Rect>
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
--- @field clipboard WmClipboard
--- @field session_lock WmSessionLock
--- @field windows WmWindows
--- @field layers WmLayers
--- @field outputs WmOutputs
--- @field text WmText
--- @field is_reload boolean
--- @field wayland_display string
--- @field is_session_locked boolean
--- @field store table
local Wm = {}

wm = Wm
--- @class Wm
--- @param delay_ms number
--- @param callback function
--- @return Timer
function Wm:after(delay_ms, callback) end

--- @class Wm
--- @param interval_ms number
--- @param callback function
--- @return Timer
function Wm:every(interval_ms, callback) end

--- @class Wm
--- @return nil
function Wm:reload() end

--- @class Wm
--- @return number
function Wm:time() end

--- @class Wm
--- @param path string
--- @param output Option<WmOutput>
--- @return nil
function Wm:screenshot(path, output) end

--- @class Wm
--- @param variables table
--- @return Array<string>
function Wm:import_environment(variables) end

--- @class Wm
--- @return nil
function Wm:quit() end

--- @param event "ready"
--- @param callback fun(e: nil)
function Wm:on(event, callback) end

--- @param event "unload"
--- @param callback fun(e: nil)
function Wm:on(event, callback) end

--- @param event "session_locked"
--- @param callback fun(e: nil)
function Wm:on(event, callback) end

--- @param event "session_unlocked"
--- @param callback fun(e: nil)
function Wm:on(event, callback) end

--- @param event "tick"
--- @param callback fun(e: nil)
function Wm:on(event, callback) end

--- @class Timer
local Timer = {}

--- @class Timer
--- @return boolean
function Timer:cancel() end

--- @alias FocusTarget Window|Layer

--- @class WmInput
--- @field seats WmSeats
local WmInput = {}

--- @class WmInput
--- @param point Point
--- @return Option<FocusTarget>
function WmInput:target_under(point) end

--- @param event "key"
--- @param callback fun(e: KeyEvent)
function WmInput:on(event, callback) end

--- @param event "shortcuts_inhibit_request"
--- @param callback fun(e: ShortcutsInhibitor)
function WmInput:on(event, callback) end

--- @param event "pointer_move"
--- @param callback fun(e: PointerMoveEvent)
function WmInput:on(event, callback) end

--- @param event "pointer_button"
--- @param callback fun(e: PointerButtonEvent)
function WmInput:on(event, callback) end

--- @param event "pointer_button_forwarded"
--- @param callback fun(e: PointerButtonEvent)
function WmInput:on(event, callback) end

--- @param event "pointer_constraint"
--- @param callback fun(e: PointerConstraint)
function WmInput:on(event, callback) end

--- @param event "pointer_gesture"
--- @param callback fun(e: PointerGestureEvent)
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
--- @return Array<Seat>
function WmSeats:all() end

--- @class WmSeats
--- @param name string
--- @return Option<Seat>
function WmSeats:get(name) end

--- @class ShortcutsInhibitor
local ShortcutsInhibitor = {}

--- @class ShortcutsInhibitor
--- @return nil
function ShortcutsInhibitor:activate() end

--- @class ShortcutsInhibitor
--- @return nil
function ShortcutsInhibitor:deactivate() end

--- @class ShortcutsInhibitor
--- @return boolean
function ShortcutsInhibitor:is_active() end

--- @class ShortcutsInhibitor
--- @return Option<Window>
function ShortcutsInhibitor:window() end

--- @class WmOutputs
local WmOutputs = {}

--- @class WmOutputs
--- @param name string
--- @param size Size
--- @return WmOutput
function WmOutputs:add_virtual(name, size) end

--- @class WmOutputs
--- @return number
function WmOutputs:count() end

--- @class WmOutputs
--- @param index number
--- @return Option<WmOutput>
function WmOutputs:get(index) end

--- @class WmOutputs
--- @return Array<WmOutput>
function WmOutputs:all() end

--- @class WmOutputs
--- @param name string
--- @return Option<WmOutput>
function WmOutputs:name(name) end

--- @class WmOutputs
--- @param point Point
--- @return Array<WmOutput>
function WmOutputs:under(point) end

--- @param event "added"
--- @param callback fun(e: WmOutput)
function WmOutputs:on(event, callback) end

--- @param event "changed"
--- @param callback fun(e: WmOutput)
function WmOutputs:on(event, callback) end

--- @param event "removed"
--- @param callback fun(e: WmOutput)
function WmOutputs:on(event, callback) end

--- @class WmClipboard
local WmClipboard = {}

--- @class WmClipboard
--- @param seat Seat
--- @return Option<ClipboardOffer>
function WmClipboard:current(seat) end

--- @class WmClipboard
--- @param seat Seat
--- @param contents table
--- @return nil
function WmClipboard:set(seat, contents) end

--- @param event "changed"
--- @param callback fun(offer: Option<ClipboardOffer>, seat: Seat)
function WmClipboard:on(event, callback) end

--- @class WmSessionLock
local WmSessionLock = {}

--- @class WmSessionLock
--- @return string
function WmSessionLock:state() end

--- @class WmSessionLock
--- @return Array<LockSurface>
function WmSessionLock:surfaces() end

--- @class WmSessionLock
--- @param color string
--- @return nil
function WmSessionLock:set_backdrop_color(color) end

--- @class LockSurface
local LockSurface = {}

--- @class LockSurface
--- @return Output
function LockSurface:output() end

--- @class LockSurface
--- @return boolean
function LockSurface:has_buffer() end

--- @class ClipboardOffer
--- @field origin string
--- @field seat Option<Seat>
local ClipboardOffer = {}

--- @class ClipboardOffer
--- @return Array<string>
function ClipboardOffer:mime_types() end

--- @class ClipboardOffer
--- @return boolean
function ClipboardOffer:is_current() end

--- @class ClipboardOffer
--- @param mime_type string
--- @param max_bytes integer
--- @param callback function
--- @return nil
function ClipboardOffer:read(mime_type, max_bytes, callback) end

--- @class Seat
--- @field name string
local Seat = {}

--- @class Seat
--- @return Option<Pointer>
function Seat:pointer() end

--- @class Seat
--- @return Option<Keyboard>
function Seat:keyboard() end

--- @class Seat
--- @return Option<FocusTarget>
function Seat:focused() end

--- @class Seat
--- @param target Option<FocusTarget>
--- @return nil
function Seat:focus(target) end

--- @class Pointer
local Pointer = {}

--- @class Pointer
--- @return Option<Seat>
function Pointer:seat() end

--- @class Pointer
--- @return Point
function Pointer:position() end

--- @class Pointer
--- @return Array<integer>
function Pointer:buttons() end

--- @class Pointer
--- @return Option<Window>
function Pointer:window_under() end

--- @class Pointer
--- @param horizontal number
--- @param vertical number
--- @param source string
--- @return nil
function Pointer:scroll(horizontal, vertical, source) end

--- @class Pointer
--- @param kind string
--- @param phase string
--- @param options table
--- @return nil
function Pointer:gesture(kind, phase, options) end

--- @class Pointer
--- @return nil
function Pointer:grab() end

--- @class Pointer
--- @return nil
function Pointer:ungrab() end

--- @class Pointer
--- @return boolean
function Pointer:grabbed() end

--- @class Pointer
--- @param point Point
--- @return nil
function Pointer:move(point) end

--- @class Pointer
--- @param button MouseButton
--- @param state "pressed"|"released"
--- @return nil
function Pointer:button(button, state) end

--- @class Keyboard
--- @param evdev_code integer linux input-event-codes.h value, e.g. 30 for A
--- @param state "pressed"|"released"
--- @return nil
function Keyboard:key(evdev_code, state) end

--- @class MouseButton
local MouseButton = {}

--- @class MouseButtons
local MouseButtons = {}

Button = MouseButtons
--- @class PointerButtonEvent
--- @field button MouseButton
--- @field state string
--- @field pointer Pointer
--- @field modifiers Modifiers
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
--- @field modifiers Modifiers
local PointerMoveEvent = {}

--- @class PointerMoveEvent
--- @return nil
function PointerMoveEvent:prevent_default() end

--- @class PointerGestureEvent
--- @field kind string
--- @field phase string
--- @field fingers number
--- @field delta Point
--- @field scale number
--- @field rotation number
--- @field cancelled boolean
--- @field pointer Pointer
--- @field modifiers Modifiers
local PointerGestureEvent = {}

--- @class PointerGestureEvent
--- @return nil
function PointerGestureEvent:prevent_default() end

--- @class PointerConstraint
local PointerConstraint = {}

--- @class PointerConstraint
--- @return boolean
function PointerConstraint:alive() end

--- @class PointerConstraint
--- @return Option<string>
function PointerConstraint:kind() end

--- @class PointerConstraint
--- @return boolean
function PointerConstraint:active() end

--- @class PointerConstraint
--- @return nil
function PointerConstraint:activate() end

--- @class PointerConstraint
--- @return nil
function PointerConstraint:deactivate() end

--- @class PointerConstraint
--- @return Pointer
function PointerConstraint:pointer() end

--- @class PointerConstraint
--- @return Option<Window>
function PointerConstraint:window() end

--- @class Keysym
local Keysym = {}

--- @class Keys
local Keys = {}

Key = Keys
--- @class Keyboard
local Keyboard = {}

--- @class Keyboard
--- @return Option<Seat>
function Keyboard:seat() end

--- @class Keyboard
--- @return string
function Keyboard:get_layout() end

--- @class Keyboard
--- @param keymap string
--- @return boolean
function Keyboard:set_layout(keymap) end

--- @class Keyboard
--- @return integer
function Keyboard:repeat_rate() end

--- @class Keyboard
--- @return integer
function Keyboard:repeat_delay() end

--- @class Keyboard
--- @param rate integer
--- @param delay integer
--- @return nil
function Keyboard:set_repeat(rate, delay) end

--- @class KeyEvent
--- @field keyboard Keyboard
--- @field state string
--- @field serial number
--- @field time number
--- @field keycode number
--- @field key Keysym
--- @field raw_key Keysym
--- @field keys Array<Keysym>
--- @field modifiers Modifiers
--- @field shortcuts_inhibited boolean
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
--- @field current boolean
--- @field preferred boolean
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
--- @return Array<OutputMode>
function WmOutput:modes() end

--- @class WmOutput
--- @param mode OutputMode
--- @return nil
function WmOutput:set_mode(mode) end

--- @class WmOutput
--- @return string
function WmOutput:transform() end

--- @class WmOutput
--- @param name string
--- @return nil
function WmOutput:set_transform(name) end

--- @class WmOutput
--- @return Point
function WmOutput:position() end

--- @class WmOutput
--- @param point Point
--- @return nil
function WmOutput:set_position(point) end

--- @class WmOutput
--- @return number
function WmOutput:scale() end

--- @class WmOutput
--- @param scale number
--- @return nil
function WmOutput:set_scale(scale) end

--- @class WmOutput
--- @return Size
function WmOutput:size() end

--- @class WmOutput
--- @return Rect
function WmOutput:usable_area() end

--- @class WmOutput
--- @return Rect
function WmOutput:area() end

--- @class WmOutput
--- @param spec ShapeSpec
--- @return Shape
function WmOutput:add_shape(spec) end

--- @class WmOutput
--- @return nil
function WmOutput:remove() end

--- @class WmWindows
local WmWindows = {}

--- @class WmWindows
--- @return Option<Window>
function WmWindows:topmost() end

--- @class WmWindows
--- @return number
function WmWindows:count() end

--- @class WmWindows
--- @param id integer
--- @return Option<Window>
function WmWindows:by_id(id) end

--- @class WmWindows
--- @param index number
--- @return Option<Window>
function WmWindows:get(index) end

--- @class WmWindows
--- @return Array<Window>
function WmWindows:all() end

--- @class WmWindows
--- @param point Point
--- @return Option<Window>
function WmWindows:under(point) end

--- @param event "new_window"
--- @param callback fun(e: Window)
function WmWindows:on(event, callback) end

--- @param event "mapped"
--- @param callback fun(e: Window)
function WmWindows:on(event, callback) end

--- @param event "resized"
--- @param callback fun(e: Window)
function WmWindows:on(event, callback) end

--- @param event "closed"
--- @param callback fun(e: Window)
function WmWindows:on(event, callback) end

--- @param event "title_changed"
--- @param callback fun(e: Window)
function WmWindows:on(event, callback) end

--- @param event "app_id_changed"
--- @param callback fun(e: Window)
function WmWindows:on(event, callback) end

--- @param event "maximize_request"
--- @param callback fun(e: Window)
function WmWindows:on(event, callback) end

--- @param event "unmaximize_request"
--- @param callback fun(e: Window)
function WmWindows:on(event, callback) end

--- @param event "unfullscreen_request"
--- @param callback fun(e: Window)
function WmWindows:on(event, callback) end

--- @param event "minimize_request"
--- @param callback fun(e: Window)
function WmWindows:on(event, callback) end

--- @param event "unminimize_request"
--- @param callback fun(e: Window)
function WmWindows:on(event, callback) end

--- @param event "parent_changed"
--- @param callback fun(e: Window)
function WmWindows:on(event, callback) end

--- @param event "decoration_request"
--- @param callback fun(e: Window)
function WmWindows:on(event, callback) end

--- @param event "move_request"
--- @param callback fun(e: WindowDragRequest)
function WmWindows:on(event, callback) end

--- @param event "resize_request"
--- @param callback fun(e: WindowDragRequest)
function WmWindows:on(event, callback) end

--- @param event "focus"
--- @param callback fun(window: Option<Window>, seat: Seat)
function WmWindows:on(event, callback) end

--- @param event "activation_request"
--- @param callback fun(window: Window, token_age_ms: integer)
function WmWindows:on(event, callback) end

--- @param event "fullscreen_request"
--- @param callback fun(window: Window, output: WmOutput?)
function WmWindows:on(event, callback) end

--- @param event "bell"
--- @param callback fun(window: Window?)
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
--- @return nil
function Window:clear_size() end

--- @class Window
--- @return Option<string>
function Window:title() end

--- @class Window
--- @return Option<string>
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
--- @return Array<Seat>
function Window:focused_seats() end

--- @class Window
--- @return boolean
function Window:visible() end

--- @class Window
--- @param visible boolean
--- @return nil
function Window:set_visible(visible) end

--- @class Window
--- @return nil
function Window:raise() end

--- @class Window
--- @return string
function Window:kind() end

--- @class Window
--- @return integer
function Window:id() end

--- @class Window
--- @return Option<Window>
function Window:parent() end

--- @class Window
--- @return Option<WmOutput>
function Window:output() end

--- @class Window
--- @return boolean
function Window:mapped() end

--- @class Window
--- @return string
function Window:layer() end

--- @class Window
--- @param layer string
--- @return nil
function Window:set_layer(layer) end

--- @class Window
--- @param width integer
--- @param color string
--- @return nil
function Window:set_border(width, color) end

--- @class Window
--- @return nil
function Window:clear_border() end

--- @class Window
--- @param mode string
--- @return nil
function Window:set_decoration(mode) end

--- @class Window
--- @param spec ShapeSpec
--- @return Shape
function Window:add_shape(spec) end

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

--- @class WindowDragRequest
--- @field window Window
--- @field pointer Pointer
--- @field edges Option<string>
local WindowDragRequest = {}

--- @class WmLayers
local WmLayers = {}

--- @class WmLayers
--- @return number
function WmLayers:count() end

--- @class WmLayers
--- @param index number
--- @return Option<Layer>
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
--- @return Option<Layer>
function WmLayers:under(point) end

--- @param event "new_surface"
--- @param callback fun(e: Layer)
function WmLayers:on(event, callback) end

--- @param event "mapped"
--- @param callback fun(e: Layer)
function WmLayers:on(event, callback) end

--- @param event "changed"
--- @param callback fun(e: Layer)
function WmLayers:on(event, callback) end

--- @param event "remove_surface"
--- @param callback fun(e: Layer)
function WmLayers:on(event, callback) end

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
--- @return Option<WmOutput>
function Layer:output() end

--- @class Layer
--- @return string
function Layer:kind() end

--- @class Layer
--- @return string
function Layer:keyboard_interactivity() end

--- @class Layer
--- @return LayerAnchor
function Layer:anchor() end

--- @class Layer
--- @return LayerMargin
function Layer:margin() end

--- @class Layer
--- @return integer
function Layer:exclusive_zone() end

--- @class Layer
--- @return Option<string>
function Layer:exclusive_edge() end

--- @class Layer
--- @return Size
function Layer:requested_size() end

--- @class Layer
--- @return nil
function Layer:close() end

--- @class Shape
local Shape = {}

--- @class Shape
--- @param spec ShapeSpec
--- @return nil
function Shape:update(spec) end

--- @class Shape
--- @return boolean
function Shape:remove() end

--- @class Shape
--- @return boolean
function Shape:alive() end

--- @class Shape
--- @return Option<string>
function Shape:kind() end

--- @class Shape
--- @return Option<Size>
function Shape:size() end

--- @class WmText
local WmText = {}

--- @class WmText
--- @param text string
--- @param font Option<FontSpec>
--- @param max_width Option<number>
--- @return TextMetrics
function WmText:measure(text, font, max_width) end

---@class FontSpec
---@field family string? a family name, or "sans-serif", "serif", "monospace"; the default is "sans-serif"
---@field file string? path of a font file, instead of a family
---@field size number? logical pixels, above 0 and at most 1024; the default is 14
---@field weight integer? 1 to 1000; the default is 400

---@class TextMetrics
---@field width number
---@field height number
---@field baseline number distance from the top to the baseline
---@field text string what would be drawn: line breaks turned into spaces, and cut with an ellipsis to max_width

---@class LayerAnchor
---@field top boolean
---@field bottom boolean
---@field left boolean
---@field right boolean

---@class LayerMargin
---@field top integer
---@field right integer
---@field bottom integer
---@field left integer

---@alias ShapeLayer "background"|"bottom"|"top"|"overlay"

---@class ShapeSpec
---@field kind "rect"|"text" required when creating, not allowed in update
---@field x integer? left edge, relative to the window's top left or the output's top left
---@field y integer?
---@field z integer? higher is in front; on a window, below 0 is behind the window's surface
---@field layer ShapeLayer? outputs only; the default is "top"
---@field visible boolean?
---@field width integer? rect only
---@field height integer? rect only
---@field fill string? rect only, "#rrggbb[aa]", or false for none
---@field radius integer? rect only, corner radius
---@field border_width integer? rect only, drawn inside the rectangle
---@field border_color string? rect only
---@field text string? text only
---@field font FontSpec? text only; fields not given keep their value
---@field color string? text only
---@field max_width integer|false? text only; longer text is cut with an ellipsis

---@class ExitReport
---@field status integer? exit code, nil when the process was killed by a signal
---@field signal integer? the signal that killed the process
---@field timed_out boolean

---@class ExecResult : ExitReport
---@field stdout string
---@field stderr string

---@class ProcessOptions
---@field env table<string, string>?
---@field cwd string?
---@field timeout number? milliseconds before the process gets SIGTERM, and SIGKILL a second later
---@field on_stdout fun(line: string)?
---@field on_stderr fun(line: string)?
---@field on_exit fun(report: ExitReport)?
---@field survives_reload boolean? keep the process running through a reload; its callbacks are dropped

--- Starts a process without waiting for it. Processes with an on_* callback are stopped on reload unless survives_reload is set.
---@param program string
---@param args string[]?
---@param options ProcessOptions?
---@return integer pid
function spawn(program, args, options) end

--- Runs a process to completion, blocking the compositor meanwhile. Use timeout to bound that.
---@param program string
---@param args string[]?
---@param options ProcessOptions?
---@return ExecResult
function exec(program, args, options) end

--- Signals a process started with spawn.
---@param pid integer
---@param signal "term"|"kill"|"int"|"hup"?
---@return boolean found
function kill(pid, signal) end
