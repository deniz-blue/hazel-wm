---@class WmOutputs
local outputs = {}

---@alias OutputEvent "added" | "removed"
---@overload fun(event: "added", callback: fun(e: WmOutputHandle))
---@overload fun(event: "removed", callback: fun(e: WmOutputHandle))
---@param event OutputEvent
function outputs:on(event, callback) end

---@class WmOutputHandle
---@field name string
---@field description string
---@field mode Mode
---@field position Point
---@field properties {  make: string, model: string, serial: string, subpixel: string }
local WmOutputHandle = {}

---@alias Mode { size: Size, refresh: number }
