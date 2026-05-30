---@meta

---@class WmInput
---@field keyboard Keyboard
---@field pointer Pointer
local input = {}

--------------------- Keyboard ---------------------

---@class Keyboard
local keyboard = {}

---@param layout string
---@return boolean success
function keyboard:set_layout(layout) end

---@return string layout
function keyboard:get_layout() end

--------------------- Pointer ---------------------

---@class Pointer
local pointer = {}

---@return Point
function pointer:position() end

---@return integer[]
function pointer:buttons() end

--------------------- Input Events ---------------------

---@alias InputEvent "keyboard" | "pointer_move" | "pointer_button"

---@param event InputEvent
---@param callback function
---@overload fun(event: "keyboard", callback: fun(e: KeyboardEvent))
---@overload fun(event: "pointer_move", callback: fun(e: PointerMotionEvent))
---@overload fun(event: "pointer_button", callback: fun(e: PointerButtonEvent))
function input:on(event, callback) end

---@alias ModifierState { alt: boolean, ctrl: boolean, shift: boolean, logo: boolean }

---@class KeyboardEvent
---@field keycode integer
---@field modifiers ModifierState
---@field state "Pressed" | "Released"
---@field time integer
---@field keysyms integer[]
local KeyboardEvent = {}

--- Prevent sending to window/client
function KeyboardEvent:prevent_default() end

---@class PointerMotionEvent
---@field position Point
local PointerMotionEvent = {}

---@class PointerButtonEvent
---@field button integer
---@field state "Pressed" | "Released"
local PointerButtonEvent = {}
