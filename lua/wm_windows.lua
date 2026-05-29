---@meta

---@class WmWindows
local windows = {}

---@alias WindowEvent "added" | "removed"
---@overload fun(event: "added", callback: fun(e: WmWindowHandle))
---@overload fun(event: "removed", callback: fun(e: WmWindowHandle))
---@param event WindowEvent
function windows:on(event, callback) end

---@class WmWindowHandle
local WmWindowHandle = {}
