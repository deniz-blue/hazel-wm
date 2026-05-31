---@meta

---@class WmInput
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

---@param event "pointer_move"
---@param callback fun(e: PointerMotionEvent)
function input:on(event, callback) end

---@param event "pointer_button"
---@param callback fun(e: PointerButtonEvent)
function input:on(event, callback) end

---@param event "key"
---@param callback fun(e: KeyboardEvent)
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
