---@meta

---@class WindowManager
---@field name string "Hazel"
---@field input WmInput
---@field outputs WmOutputs
---@field windows WmWindows
wm = {}

---@param event "ready"
---@param callback fun()
function wm:on(event, callback) end
