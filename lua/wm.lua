---@meta

---@class WindowManager
---@field name string "Hazel"
---@field input WmInput
---@field outputs WmOutputs
---@field windows WmWindows
wm = {}

---@alias WmEvent "ready" | "commit"

---@overload fun(event: "ready", callback: fun())
---@overload fun(event: "commit", callback: fun())
---@param event WmEvent
function wm:on(event, callback) end
