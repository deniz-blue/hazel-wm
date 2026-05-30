---@meta

---@class WmOutputs
local outputs = {}

---@alias OutputEvent "added" | "removed"
---@overload fun(event: "added", callback: fun(e: WmOutputHandle))
---@overload fun(event: "removed", callback: fun(e: WmOutputHandle))
---@param event OutputEvent
function outputs:on(event, callback) end

---Return an output by its name, or nil if it doesn't exist
---@param name string
---@return WmOutputHandle | nil
function outputs:name(name) end

---Amount of outputs
---@return integer
function outputs:count() end

---@alias Mode { size: Size, refresh: number }

---@class WmOutputHandle
---@field name string
---@field description string
---@field mode Mode
---@field properties { make: string, model: string, serial: string, subpixel: string }
local WmOutputHandle = {}

---@return Point
function WmOutputHandle:position() end

---Move the output to a new position
---@param pos Point
function WmOutputHandle:set_position(pos) end
