---@meta

---@class WindowManager
---@field name string "Hazel"
---@field input WmInput
wm = {}

---@alias WmEvent "ready" | "commit"

---@overload fun(event: "ready", callback: fun())
---@overload fun(event: "commit", callback: fun())
---@param event WmEvent
function wm:on(event, callback) end

---@class WmInput
local input = {}

---@alias InputEvent "keyboard"

---@overload fun(event: "keyboard", callback: fun(e: KeyboardEvent))
---@param event InputEvent
function input:on(event, callback) end

---@class KeyboardEvent
---@field keycode integer
local KeyboardEvent = {}

--- Prevent sending to window/client
function KeyboardEvent:prevent_default() end

--- Spawn a process and forget about it
---@param command string
---@param args string[] | nil
function spawn(command, args) end
