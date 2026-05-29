---@meta

---@class WmInput
local input = {}

---@alias InputEvent "keyboard"

---@param event InputEvent
---@param callback function
---@overload fun(event: "keyboard", callback: fun(e: KeyboardEvent))
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
